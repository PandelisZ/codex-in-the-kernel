#!/usr/bin/env python3
import argparse
import json
import os
import pathlib
import queue
import re
import select
import shutil
import subprocess
import sys
import termios
import textwrap
import threading
import tty
import unicodedata
import venv
from dataclasses import dataclass
from enum import Enum
from typing import Any

import curses


HARNESS_ROOT = pathlib.Path(__file__).resolve().parents[2]
DEFAULT_TOKEN_PATH = HARNESS_ROOT / "artifacts" / "rootfs" / "etc" / "cilux" / "ws-token"
DEFAULT_WS_URL = "ws://127.0.0.1:8765"
DEFAULT_CWD = "/workspace"


def ensure_websockets():
    try:
        from websockets.sync.client import connect  # type: ignore
    except ImportError:
        venv_dir = HARNESS_ROOT / "tests" / ".venv"
        if not venv_dir.exists():
            venv.EnvBuilder(with_pip=True).create(venv_dir)
        venv_python = venv_dir / "bin" / "python3"
        subprocess.check_call([str(venv_python), "-m", "pip", "install", "websockets"])
        os.execv(str(venv_python), [str(venv_python), *sys.argv])
    return connect


connect = ensure_websockets()


@dataclass
class TranscriptEntry:
    prefix: str
    text: str


@dataclass
class RenderSpan:
    text: str
    attr: int = 0


@dataclass
class RenderLine:
    spans: list[RenderSpan]


@dataclass
class UiEvent:
    kind: str
    payload: Any = None


@dataclass
class PickerOption:
    label: str
    value: str | None = None
    description: str = ""


@dataclass
class PickerState:
    kind: str
    title: str
    options: list[PickerOption]
    selected: int = 0
    help_text: str = ""


class ReasoningEffort(str, Enum):
    none = "none"
    minimal = "minimal"
    low = "low"
    medium = "medium"
    high = "high"
    xhigh = "xhigh"


class AppServerSession:
    def __init__(
        self,
        ws_url: str,
        token: str,
        *,
        cwd: str,
        approval_policy: str,
        sandbox: str,
        personality: str,
        model: str | None,
    ) -> None:
        self.ws_url = ws_url
        self.token = token
        self.cwd = cwd
        self.approval_policy = approval_policy
        self.sandbox = sandbox
        self.personality = personality
        self.model = model
        self.ws = None
        self.next_id = 1
        self.pending: list[dict[str, Any]] = []
        self.thread_id: str | None = None
        self.last_status = "idle"
        self.last_usage = ""
        self.active_turn_id: str | None = None

    def __enter__(self) -> "AppServerSession":
        kwargs = {"max_size": None}
        try:
            self.ws = connect(
                self.ws_url,
                additional_headers={"Authorization": f"Bearer {self.token}"},
                **kwargs,
            )
        except TypeError:
            self.ws = connect(
                self.ws_url,
                extra_headers={"Authorization": f"Bearer {self.token}"},
                **kwargs,
            )
        self._request(
            "initialize",
            {
                "clientInfo": {
                    "name": "cilux_tui",
                    "title": "Cilux TUI",
                    "version": "0.1.0",
                }
            },
        )
        self._notify("initialized", {})
        self.start_thread()
        return self

    def __exit__(self, exc_type, exc, tb) -> None:
        if self.ws is not None:
            self.ws.close()
            self.ws = None

    def start_thread(self) -> dict[str, Any]:
        params: dict[str, Any] = {
            "cwd": self.cwd,
            "approvalPolicy": self.approval_policy,
            "sandbox": self.sandbox,
            "personality": self.personality,
        }
        if self.model:
            params["model"] = self.model
        result = self._request("thread/start", params)
        self.thread_id = result["thread"]["id"]
        return result

    def turn(
        self,
        prompt: str,
        on_event,
        *,
        optimistic_agent_entry: TranscriptEntry | None = None,
        model: str | None = None,
        effort: str | None = None,
        tools: dict[str, Any] | None = None,
    ) -> None:
        if self.thread_id is None:
            raise RuntimeError("thread has not been started")

        params: dict[str, Any] = {
            "threadId": self.thread_id,
            "input": [
                {
                    "type": "text",
                    "text": prompt,
                    "textElements": [],
                }
            ],
        }
        if model:
            params["model"] = model
        if effort:
            params["effort"] = effort
        if tools is not None:
            params["tools"] = tools

        result = self._request("turn/start", params)
        turn_id = result["turn"]["id"]
        self.active_turn_id = turn_id
        agent_entries: dict[str, TranscriptEntry] = {}
        command_streamed_output: set[str] = set()

        while True:
            message = self._next_message()

            if "id" in message and "method" not in message:
                continue

            method = message.get("method")
            params = message.get("params", {})
            if method is None:
                continue

            if method == "item/started":
                item = params.get("item", {})
                item_type = item.get("type")
                if item_type == "agentMessage":
                    phase = item.get("phase", "final_answer")
                    prefix = "assistant"
                    if phase != "final_answer":
                        prefix = f"assistant:{phase}"
                    if optimistic_agent_entry is not None and not agent_entries:
                        entry = optimistic_agent_entry
                        entry.prefix = prefix
                        entry.text = ""
                        on_event("refresh", None)
                    else:
                        entry = TranscriptEntry(prefix=prefix, text="")
                        on_event("entry", entry)
                    agent_entries[item["id"]] = entry
                    on_event("assistant_start", prefix)
                elif item_type == "mcpToolCall":
                    on_event(
                        "system",
                        f"[tool:start] {item.get('tool')} {format_args(item.get('arguments'))}",
                    )
                elif item_type == "commandExecution":
                    on_event("system", f"[cmd:start] {item.get('command')}")
            elif method == "item/agentMessage/delta" and params.get("turnId") == turn_id:
                item_id = params.get("itemId")
                entry = agent_entries.get(item_id)
                if entry is not None:
                    delta = params.get("delta", "")
                    entry.text += delta
                    on_event("assistant_delta", delta)
                    on_event("refresh", None)
            elif method == "item/completed":
                item = params.get("item", {})
                item_type = item.get("type")
                if item_type == "mcpToolCall":
                    result = item.get("result")
                    error = item.get("error")
                    if error:
                        on_event("system", f"[tool:error] {item.get('tool')} {error}")
                    elif isinstance(result, dict) and result.get("isError"):
                        text = extract_mcp_text(result)
                        on_event("system", f"[tool:error] {item.get('tool')} {text}")
                    else:
                        on_event("system", f"[tool:done] {item.get('tool')}")
                elif item_type == "commandExecution":
                    exit_code = item.get("exitCode")
                    on_event("system", f"[cmd:done] exit={exit_code} {item.get('command')}")
                    aggregated = item.get("aggregatedOutput")
                    if aggregated and item.get("id") not in command_streamed_output:
                        for line in aggregated.rstrip().splitlines():
                            on_event("system", f"  {line}")
                elif item_type == "agentMessage":
                    on_event("assistant_end", item)
            elif method == "item/commandExecution/outputDelta" and params.get("turnId") == turn_id:
                delta = params.get("delta", "")
                if delta:
                    command_streamed_output.add(params.get("itemId", ""))
                    for line in delta.rstrip("\n").splitlines():
                        on_event("system", f"  {line}")
            elif method == "item/mcpToolCall/progress" and params.get("turnId") == turn_id:
                on_event("system", f"[tool:progress] {params.get('message')}")
            elif method == "thread/status/changed":
                status = params.get("status", {})
                self.last_status = str(status.get("type", self.last_status))
                on_event("refresh", None)
            elif method == "thread/tokenUsage/updated":
                usage = params.get("tokenUsage", {}).get("last", {})
                if usage:
                    self.last_usage = (
                        f"in={usage.get('inputTokens', 0)} "
                        f"out={usage.get('outputTokens', 0)} "
                        f"reason={usage.get('reasoningOutputTokens', 0)} "
                        f"total={usage.get('totalTokens', 0)}"
                    )
                    on_event("refresh", None)
            elif method == "mcpServer/startupStatus/updated":
                on_event(
                    "system",
                    f"[mcp:{params.get('name')}] {params.get('status')}"
                    + (f" error={params.get('error')}" if params.get("error") else ""),
                )
            elif method == "configWarning":
                on_event("system", f"[config-warning] {params.get('summary')}")
            elif method == "turn/completed" and params.get("turn", {}).get("id") == turn_id:
                turn = params.get("turn", {})
                self.last_status = turn.get("status", self.last_status)
                error = turn.get("error")
                self.active_turn_id = None
                if error:
                    on_event("system", f"[turn:error] {error}")
                on_event("turn_completed", {"status": self.last_status, "error": error})
                on_event("refresh", None)
                return

    def _request(self, method: str, params: dict[str, Any]) -> dict[str, Any]:
        request_id = self.next_id
        self.next_id += 1
        self._send({"id": request_id, "method": method, "params": params})
        deferred: list[dict[str, Any]] = []

        while True:
            if self.pending:
                message = self.pending.pop(0)
            else:
                message = self._recv_message()

            if "method" in message and "id" in message:
                self._handle_server_request(message)
                continue

            if "method" in message and "id" not in message:
                deferred.append(message)
                continue

            if message.get("id") != request_id:
                continue

            if "error" in message:
                raise RuntimeError(f"{method} failed: {message['error']}")
            if deferred:
                self.pending = deferred + self.pending
            return message["result"]

    def _notify(self, method: str, params: dict[str, Any]) -> None:
        self._send({"method": method, "params": params})

    def _handle_server_request(self, message: dict[str, Any]) -> None:
        # The guest thread runs in danger-full-access, so approval requests
        # should not normally appear. Reply with an empty result rather than
        # deadlocking the connection if one does.
        self._send({"id": message["id"], "result": {}})

    def _next_message(self) -> dict[str, Any]:
        if self.pending:
            return self.pending.pop(0)
        return self._recv_message()

    def _recv_message(self) -> dict[str, Any]:
        assert self.ws is not None
        raw = self.ws.recv()
        if raw is None:
            raise RuntimeError("websocket closed")
        return json.loads(raw)

    def _send(self, payload: dict[str, Any]) -> None:
        assert self.ws is not None
        self.ws.send(json.dumps(payload))

    def list_models(self) -> list[dict[str, Any]]:
        result = self._request("model/list", {})
        return result.get("data", [])

    def interrupt_active_turn(self) -> bool:
        if self.thread_id is None or self.active_turn_id is None:
            return False
        request_id = self.next_id
        self.next_id += 1
        self._send(
            {
                "id": request_id,
                "method": "turn/interrupt",
                "params": {
                    "threadId": self.thread_id,
                    "turnId": self.active_turn_id,
                },
            }
        )
        return True


class TuiApp:
    def __init__(self, session: AppServerSession) -> None:
        self.session = session
        self.transcript: list[TranscriptEntry] = []
        self.input_buffer = ""
        self.running = True
        self.event_queue: queue.SimpleQueue[UiEvent] = queue.SimpleQueue()
        self.turn_thread: threading.Thread | None = None
        self.turn_active = False
        self.pending_agent_entry: TranscriptEntry | None = None
        self.current_model: str | None = None
        self.current_effort: str | None = None
        self.current_tools: dict[str, Any] = {"view_image": True, "web_search": None}
        self.model_cache: list[dict[str, Any]] | None = None
        self.panel: PickerState | None = None

    def run(self, stdscr) -> None:
        curses.curs_set(1)
        stdscr.keypad(True)
        curses.use_default_colors()
        stdscr.timeout(50)

        self._add_system("Connected to guest Codex app-server")
        self._add_system(f"Thread: {self.session.thread_id}")
        self._add_system(
            "Commands: /new, /clear, /quit, /models, /model <id>, /reasoning <level|default>, /tools, /tool <web_search|view_image> <on|off>"
        )

        while self.running:
            self._drain_events()
            self._draw(stdscr)
            try:
                ch = stdscr.get_wch()
            except curses.error:
                continue

            if self.panel is not None:
                if self._handle_panel_input(ch):
                    continue

            if ch in ("\n", "\r"):
                text = self.input_buffer.strip()
                self.input_buffer = ""
                if not text:
                    continue
                if text in {"/quit", "/exit"}:
                    if self.turn_active:
                        self._add_system("A turn is still running; wait for completion before exiting.")
                        continue
                    self.running = False
                    continue
                if text == "/clear":
                    self.transcript.clear()
                    self._add_system("Transcript cleared")
                    continue
                if text == "/new":
                    if self.turn_active:
                        self._add_system("Cannot start a new thread while a turn is in progress.")
                        continue
                    result = self.session.start_thread()
                    self.transcript.clear()
                    self._add_system(f"Started new thread: {result['thread']['id']}")
                    continue
                if self.turn_active and text.startswith("/"):
                    self._add_system("Cannot change settings while a turn is in progress.")
                    continue
                if text.startswith("/"):
                    self._handle_slash_command(text)
                    continue
                if self.turn_active:
                    self._add_system("A turn is already running. Wait for it to finish before sending another prompt.")
                    continue
                self._add_entry("you", text)
                self._start_turn(text)
            elif ch in ("\b", "\x7f") or ch == curses.KEY_BACKSPACE:
                self.input_buffer = self.input_buffer[:-1]
            elif ch == curses.KEY_RESIZE:
                continue
            elif isinstance(ch, str) and ch.isprintable():
                self.input_buffer += ch

        self._drain_events()

    def _start_turn(self, text: str) -> None:
        self.turn_active = True
        self.session.last_status = "submitting"
        self.pending_agent_entry = TranscriptEntry(prefix="assistant:pending", text="...")
        self.transcript.append(self.pending_agent_entry)
        self._add_system(
            "[turn] started"
            + (f" model={self.current_model}" if self.current_model else "")
            + (f" effort={self.current_effort}" if self.current_effort else "")
            + f" tools={self._tools_summary()}"
        )

        def worker() -> None:
            try:
                self.session.turn(
                    text,
                    lambda kind, payload: self.event_queue.put(UiEvent(kind, payload)),
                    optimistic_agent_entry=self.pending_agent_entry,
                    model=self.current_model,
                    effort=self.current_effort,
                    tools=self._turn_tools_payload(),
                )
            except Exception as exc:  # pragma: no cover - defensive UI path
                self.event_queue.put(UiEvent("system", f"[turn:error] {exc}"))
            finally:
                self.event_queue.put(UiEvent("turn_finished"))

        self.turn_thread = threading.Thread(target=worker, daemon=True)
        self.turn_thread.start()

    def _drain_events(self) -> None:
        while True:
            try:
                event = self.event_queue.get_nowait()
            except queue.Empty:
                return
            self._on_event(event.kind, event.payload)

    def _on_event(self, kind: str, payload: Any) -> None:
        if kind == "entry":
            assert isinstance(payload, TranscriptEntry)
            self.transcript.append(payload)
        elif kind == "system":
            self._add_system(str(payload))
        elif kind == "refresh":
            pass
        elif kind == "turn_finished":
            self.turn_active = False
            self.pending_agent_entry = None

    def _add_entry(self, prefix: str, text: str) -> None:
        self.transcript.append(TranscriptEntry(prefix=prefix, text=text))

    def _add_system(self, text: str) -> None:
        self._add_entry("system", text)

    def _handle_slash_command(self, text: str) -> None:
        parts = text.split()
        command = parts[0].lower()

        if command == "/models":
            self._open_model_picker(force_refresh=True)
            return

        if command == "/model":
            if len(parts) == 1:
                self._open_model_picker(force_refresh=False)
                return
            value = parts[1]
            if value in {"default", "reset"}:
                self.current_model = None
                self._add_system("[model] reset to server default")
            else:
                self.current_model = value
                self._add_system(f"[model] set to {value}")
            return

        if command == "/reasoning":
            if len(parts) == 1:
                self._open_reasoning_picker()
                return
            value = parts[1].lower()
            if value in {"default", "reset"}:
                self.current_effort = None
                self._add_system("[reasoning] reset to server default")
                return
            if value not in {item.value for item in ReasoningEffort}:
                self._add_system("[reasoning] expected one of: none minimal low medium high xhigh default")
                return
            self.current_effort = value
            self._add_system(f"[reasoning] set to {value}")
            return

        if command == "/tools":
            self._open_tools_picker()
            return

        if command == "/tool":
            if len(parts) == 1:
                self._open_tools_picker()
                return
            if len(parts) < 3:
                self._add_system("[tool] expected /tool <web_search|view_image> <on|off>")
                return
            tool_name = parts[1].lower()
            toggle = parts[2].lower()
            if tool_name not in {"web_search", "view_image"}:
                self._add_system("[tool] supported tools: web_search, view_image")
                return
            if toggle not in {"on", "off"}:
                self._add_system("[tool] toggle must be on or off")
                return
            enabled = toggle == "on"
            if tool_name == "view_image":
                self.current_tools["view_image"] = enabled
            elif tool_name == "web_search":
                self.current_tools["web_search"] = {} if enabled else None
            self._add_system(f"[tool] {tool_name} -> {toggle}")
            return

        self._add_system(f"[command] unknown slash command: {text}")

    def _open_model_picker(self, *, force_refresh: bool) -> None:
        if force_refresh or self.model_cache is None:
            models = self.session.list_models()
            self.model_cache = models
        else:
            models = self.model_cache

        options = [PickerOption(label="Server default", value="__default__", description="Use the server-selected default model")]
        for model in models:
            model_id = model.get("id", model.get("model", "?"))
            display_name = model.get("displayName", model_id)
            efforts = ",".join(
                option.get("reasoningEffort", "?")
                for option in model.get("supportedReasoningEfforts", [])
            )
            desc = f"default_effort={model.get('defaultReasoningEffort', '?')} efforts={efforts}"
            options.append(PickerOption(label=f"{display_name} [{model_id}]", value=model_id, description=desc))

        selected = 0
        if self.current_model:
            for idx, option in enumerate(options):
                if option.value == self.current_model:
                    selected = idx
                    break

        self.panel = PickerState(
            kind="model",
            title="Model Picker",
            options=options,
            selected=selected,
            help_text="Enter selects model. Esc closes.",
        )

    def _open_reasoning_picker(self) -> None:
        options = [PickerOption(label="Server default", value="__default__", description="Use the model/server default reasoning effort")]
        for effort in ReasoningEffort:
            options.append(
                PickerOption(
                    label=effort.value,
                    value=effort.value,
                    description=f"Set reasoning effort to {effort.value}",
                )
            )

        selected = 0
        if self.current_effort:
            for idx, option in enumerate(options):
                if option.value == self.current_effort:
                    selected = idx
                    break

        self.panel = PickerState(
            kind="reasoning",
            title="Reasoning Picker",
            options=options,
            selected=selected,
            help_text="Enter selects effort. Esc closes.",
        )

    def _open_tools_picker(self) -> None:
        options = [
            PickerOption(
                label=f"web_search: {'on' if self.current_tools.get('web_search') is not None else 'off'}",
                value="web_search",
                description="Toggle web search tool exposure",
            ),
            PickerOption(
                label=f"view_image: {'on' if self.current_tools.get('view_image') else 'off'}",
                value="view_image",
                description="Toggle image viewer tool exposure",
            ),
        ]
        self.panel = PickerState(
            kind="tools",
            title="Tool Toggles",
            options=options,
            selected=0,
            help_text="Enter toggles selected tool. Esc closes.",
        )

    def _handle_panel_input(self, ch: Any) -> bool:
        assert self.panel is not None
        if ch in ("\x1b",):
            self.panel = None
            return True
        if ch in (curses.KEY_UP, "k"):
            self.panel.selected = max(0, self.panel.selected - 1)
            return True
        if ch in (curses.KEY_DOWN, "j"):
            self.panel.selected = min(len(self.panel.options) - 1, self.panel.selected + 1)
            return True
        if ch in ("\n", "\r", " "):
            self._apply_panel_selection()
            return True
        return False

    def _apply_panel_selection(self) -> None:
        assert self.panel is not None
        option = self.panel.options[self.panel.selected]

        if self.panel.kind == "model":
            if option.value == "__default__":
                self.current_model = None
                self._add_system("[model] reset to server default")
            else:
                self.current_model = option.value
                self._add_system(f"[model] set to {option.value}")
            self.panel = None
            return

        if self.panel.kind == "reasoning":
            if option.value == "__default__":
                self.current_effort = None
                self._add_system("[reasoning] reset to server default")
            else:
                self.current_effort = option.value
                self._add_system(f"[reasoning] set to {option.value}")
            self.panel = None
            return

        if self.panel.kind == "tools":
            if option.value == "view_image":
                self.current_tools["view_image"] = not bool(self.current_tools.get("view_image"))
            elif option.value == "web_search":
                self.current_tools["web_search"] = None if self.current_tools.get("web_search") is not None else {}
            self._open_tools_picker()
            self._add_system(f"[tools] {self._tools_summary()}")
            return

    def _tools_summary(self) -> str:
        web_search_enabled = self.current_tools.get("web_search") is not None
        view_image_enabled = bool(self.current_tools.get("view_image"))
        return f"web_search={'on' if web_search_enabled else 'off'}, view_image={'on' if view_image_enabled else 'off'}"

    def _turn_tools_payload(self) -> dict[str, Any]:
        return {
            "view_image": bool(self.current_tools.get("view_image")),
            "web_search": self.current_tools.get("web_search"),
        }

    def _draw(self, stdscr) -> None:
        stdscr.erase()
        height, width = stdscr.getmaxyx()
        title = (
            f"Cilux Guest Codex TUI | {self.session.ws_url} | "
            f"thread={shorten(self.session.thread_id)} | status={self.session.last_status} | "
            f"model={self.current_model or 'default'} | effort={self.current_effort or 'default'}"
        )
        usage = self.session.last_usage or "no usage yet"
        panel_height = self._panel_height()
        footer = (
            "Prompt or /command. /models, /model, /reasoning, /tools, /tool, /new, /clear, /quit."
        )
        if self.turn_active:
            footer = f"Streaming turn in progress. New input disabled. tools={self._tools_summary()}"

        stdscr.addnstr(0, 0, title, width - 1, curses.A_REVERSE)
        stdscr.addnstr(1, 0, usage, width - 1)

        transcript_height = max(1, height - 4 - panel_height)
        lines: list[RenderLine] = []
        for entry in self.transcript:
            wrapped = render_entry(entry, width)
            lines.extend(wrapped)
        visible = lines[-transcript_height:]
        for idx, line in enumerate(visible, start=2):
            draw_render_line(stdscr, idx, 0, line, width - 1)

        footer_y = height - 2 - panel_height
        input_y = height - 1 - panel_height
        stdscr.addnstr(footer_y, 0, footer, width - 1, curses.A_DIM)
        prompt = f"you> {self.input_buffer}"
        stdscr.addnstr(input_y, 0, prompt, width - 1)
        self._draw_panel(stdscr, input_y + 1, width)
        stdscr.move(input_y, min(len(prompt), width - 1))
        stdscr.refresh()

    def _panel_height(self) -> int:
        if self.panel is None:
            return 0
        visible_options = min(len(self.panel.options), 6)
        return 2 + visible_options

    def _draw_panel(self, stdscr, start_y: int, width: int) -> None:
        if self.panel is None:
            return

        panel_height = self._panel_height()
        title = f"[{self.panel.title}] {self.panel.help_text}"
        stdscr.addnstr(start_y, 0, title, width - 1, curses.A_STANDOUT)

        visible_rows = panel_height - 2
        offset = 0
        if self.panel.selected >= visible_rows:
            offset = self.panel.selected - visible_rows + 1

        options = self.panel.options[offset : offset + visible_rows]
        for row_idx, option in enumerate(options, start=1):
            absolute_idx = offset + row_idx - 1
            marker = ">" if absolute_idx == self.panel.selected else " "
            text = f"{marker} {option.label}"
            attr = curses.A_BOLD if absolute_idx == self.panel.selected else 0
            stdscr.addnstr(start_y + row_idx, 0, text, width - 1, attr)


class ShellTurnPrinter:
    def __init__(self) -> None:
        self.line_open = False
        self.last_channel: str | None = None

    def assistant_start(self, prefix: str) -> None:
        self._flush_open_line()
        label = "assistant"
        if prefix.startswith("assistant"):
            label = "assistant"
        elif prefix:
            label = prefix
        sys.stdout.write(f"{label}> ")
        sys.stdout.flush()
        self.line_open = True
        self.last_channel = "assistant"

    def assistant_delta(self, delta: str) -> None:
        if not delta:
            return
        sys.stdout.write(delta)
        sys.stdout.flush()
        self.line_open = not delta.endswith("\n")

    def assistant_end(self) -> None:
        self._flush_open_line()

    def system(self, message: str) -> None:
        self._flush_open_line()
        if self.last_channel == "system":
            print(f"{' ' * 8}{message}")
        else:
            print(f"system> {message}")
        self.last_channel = "system"

    def _flush_open_line(self) -> None:
        if self.line_open:
            sys.stdout.write("\n")
            sys.stdout.flush()
            self.line_open = False


class ShellTerminal:
    def __init__(self) -> None:
        self.fd = sys.stdin.fileno()
        self.old_attrs = None
        self.panel_line_count = 0

    def __enter__(self) -> "ShellTerminal":
        if not sys.stdin.isatty():
            raise RuntimeError("stdin is not a TTY")
        self.old_attrs = termios.tcgetattr(self.fd)
        tty.setcbreak(self.fd)
        return self

    def __exit__(self, exc_type, exc, tb) -> None:
        self.clear_transient_area()
        if self.old_attrs is not None:
            termios.tcsetattr(self.fd, termios.TCSADRAIN, self.old_attrs)

    def render(self, prompt: str, panel_lines: list[str], cursor_offset: int) -> None:
        self.clear_transient_area()
        columns = shutil.get_terminal_size(fallback=(120, 30)).columns
        prompt = clip_display_text(prompt, max(1, columns - 1))
        sys.stdout.write("\r\033[2K")
        sys.stdout.write(prompt)
        for line in panel_lines:
            line = clip_display_text(line, max(1, columns - 1))
            sys.stdout.write("\n\033[2K")
            sys.stdout.write(line)
        if panel_lines:
            sys.stdout.write(f"\033[{len(panel_lines)}A\r")
        sys.stdout.write(f"\033[{max(0, min(cursor_offset, columns - 1))}C")
        sys.stdout.flush()
        self.panel_line_count = len(panel_lines)

    def clear_transient_area(self) -> None:
        sys.stdout.write("\r\033[2K")
        for _ in range(self.panel_line_count):
            sys.stdout.write("\n\033[2K")
        if self.panel_line_count:
            sys.stdout.write(f"\033[{self.panel_line_count}A\r")
        sys.stdout.flush()
        self.panel_line_count = 0

    def suspend(self) -> None:
        self.clear_transient_area()
        sys.stdout.write("\r\033[2K")
        sys.stdout.flush()

    def read_key(self, timeout: float = 0.1) -> str | None:
        if not select.select([self.fd], [], [], timeout)[0]:
            return None
        first = os.read(self.fd, 1)
        if first == b"\x03":
            raise KeyboardInterrupt
        if first in {b"\r", b"\n"}:
            return "ENTER"
        if first in {b"\x7f", b"\b"}:
            return "BACKSPACE"
        if first == b"\x1b":
            seq = b""
            while select.select([self.fd], [], [], 0.005)[0]:
                seq += os.read(self.fd, 1)
                if len(seq) >= 2 and seq[:1] == b"[" and seq[1:2] in {b"A", b"B", b"C", b"D"}:
                    break
            if seq == b"[A":
                return "UP"
            if seq == b"[B":
                return "DOWN"
            if seq == b"[C":
                return "RIGHT"
            if seq == b"[D":
                return "LEFT"
            return "ESC"
        try:
            return first.decode("utf-8")
        except UnicodeDecodeError:
            return ""


class ShellApp:
    def __init__(self, session: AppServerSession) -> None:
        self.session = session
        self.current_model: str | None = None
        self.current_effort: str | None = None
        self.current_tools: dict[str, Any] = {"view_image": True, "web_search": None}
        self.model_cache: list[dict[str, Any]] | None = None
        self.input_buffer = ""
        self.cursor_pos = 0
        self.panel: PickerState | None = None
        self.term: ShellTerminal | None = None
        self.event_queue: queue.SimpleQueue[UiEvent] = queue.SimpleQueue()
        self.turn_thread: threading.Thread | None = None
        self.turn_active = False
        self.printer = ShellTurnPrinter()

    def run(self) -> None:
        print("Connected to guest Codex app-server")
        print(f"Thread: {self.session.thread_id}")
        print(
            "Commands: /new, /clear, /quit, /models, /model, /reasoning, /tools, /tool, /settings"
        )

        with ShellTerminal() as term:
            self.term = term
            while True:
                self._drain_events()
                self._render()
                key = term.read_key(timeout=0.05)
                if key is None:
                    continue

                if self.panel is not None and self._handle_panel_key(key):
                    continue

                if key == "ENTER":
                    text = self.input_buffer.strip()
                    self.input_buffer = ""
                    self.cursor_pos = 0

                    if not text:
                        continue
                    if text in {"/quit", "/exit"}:
                        self._suspend()
                        print()
                        return
                    if text == "/clear":
                        self._suspend()
                        subprocess.run(["clear"], check=False)
                        continue
                    if text == "/new":
                        if self.turn_active:
                            self._suspend()
                            print("system> cannot start a new thread while a turn is in progress")
                            continue
                        self._suspend()
                        result = self.session.start_thread()
                        print(f"system> started new thread: {result['thread']['id']}")
                        continue
                    if self.turn_active:
                        self._suspend()
                        print("system> a turn is already running; press Esc to interrupt it")
                        continue
                    if text.startswith("/"):
                        self._handle_command(text)
                        continue
                    self._run_turn(text)
                    continue

                if key == "BACKSPACE":
                    if self.cursor_pos > 0:
                        self.input_buffer = (
                            self.input_buffer[: self.cursor_pos - 1]
                            + self.input_buffer[self.cursor_pos :]
                        )
                        self.cursor_pos -= 1
                    continue

                if key == "LEFT":
                    self.cursor_pos = max(0, self.cursor_pos - 1)
                    continue
                if key == "RIGHT":
                    self.cursor_pos = min(len(self.input_buffer), self.cursor_pos + 1)
                    continue
                if key == "ESC":
                    if self.panel is not None:
                        self.panel = None
                    elif self.turn_active:
                        self._interrupt_turn()
                    continue
                if len(key) == 1 and key.isprintable():
                    self.input_buffer = (
                        self.input_buffer[: self.cursor_pos]
                        + key
                        + self.input_buffer[self.cursor_pos :]
                    )
                    self.cursor_pos += 1

    def _prompt_label(self) -> str:
        model = self.current_model or "default"
        effort = self.current_effort or "default"
        return f"you [{model} • {effort} • {self._tools_prompt_summary()}]> "

    def _render(self) -> None:
        assert self.term is not None
        prompt_label = self._prompt_label()
        columns = shutil.get_terminal_size(fallback=(120, 30)).columns
        visible_input, input_cursor = fit_input_to_width(
            self.input_buffer,
            self.cursor_pos,
            max(8, columns - display_width(prompt_label) - 1),
        )
        prompt = prompt_label + visible_input
        panel_lines = self._panel_lines()
        cursor_offset = display_width(prompt_label) + input_cursor
        self.term.render(prompt, panel_lines, cursor_offset)

    def _suspend(self) -> None:
        assert self.term is not None
        self.term.suspend()

    def _run_turn(self, text: str) -> None:
        self._suspend()
        print()
        print(f"you> {text}")
        print(
            "system> "
            f"turn started with model={self.current_model or 'default'}, "
            f"effort={self.current_effort or 'default'}, "
            f"tools={self._tools_summary()}"
        )
        print("        press Esc to interrupt")

        self.turn_active = True
        self.session.last_status = "inProgress"

        def worker() -> None:
            def on_event(kind: str, payload: Any) -> None:
                self.event_queue.put(UiEvent(kind, payload))

            try:
                self.session.turn(
                    text,
                    on_event,
                    model=self.current_model,
                    effort=self.current_effort,
                    tools=self._turn_tools_payload(),
                )
            except Exception as exc:  # pragma: no cover - defensive UI path
                self.event_queue.put(UiEvent("system", f"turn failed: {exc}"))
                self.event_queue.put(UiEvent("turn_completed", {"status": "failed", "error": str(exc)}))

        self.turn_thread = threading.Thread(target=worker, daemon=True)
        self.turn_thread.start()

    def _interrupt_turn(self) -> None:
        self._suspend()
        if self.session.interrupt_active_turn():
            print("system> interrupt requested")
        else:
            print("system> no active turn to interrupt")

    def _drain_events(self) -> None:
        while True:
            try:
                event = self.event_queue.get_nowait()
            except queue.Empty:
                return
            kind = event.kind
            payload = event.payload
            if kind == "assistant_start":
                self._suspend()
                self.printer.assistant_start(str(payload))
            elif kind == "assistant_delta":
                self.printer.assistant_delta(str(payload))
            elif kind == "assistant_end":
                self.printer.assistant_end()
            elif kind == "system":
                self._suspend()
                self.printer.system(str(payload))
            elif kind == "turn_completed":
                self._suspend()
                self.printer.assistant_end()
                status = payload.get("status", "unknown")
                error = payload.get("error")
                print(f"system> turn completed: {status}")
                if error:
                    print(f"        turn error: {error}")
                print()
                self.turn_active = False
            elif kind == "refresh":
                pass

    def _handle_command(self, text: str) -> None:
        parts = text.split()
        command = parts[0].lower()

        if command == "/settings":
            self._suspend()
            print(
                "system> "
                f"model={self.current_model or 'default'} | "
                f"effort={self.current_effort or 'default'} | "
                f"tools={self._tools_summary()}"
            )
            return

        if command == "/models":
            self._open_model_picker(force_refresh=True)
            return

        if command == "/model":
            if len(parts) > 1:
                self._suspend()
                value = parts[1]
                if value in {"default", "reset"}:
                    self.current_model = None
                    print("system> model reset to server default")
                else:
                    self.current_model = value
                    print(f"system> model set to {value}")
                return
            self._open_model_picker(force_refresh=False)
            return

        if command == "/reasoning":
            if len(parts) > 1:
                self._suspend()
                value = parts[1].lower()
                if value in {"default", "reset"}:
                    self.current_effort = None
                    print("system> reasoning reset to server default")
                    return
                if value not in {item.value for item in ReasoningEffort}:
                    print("system> expected one of: none minimal low medium high xhigh default")
                    return
                self.current_effort = value
                print(f"system> reasoning set to {value}")
                return
            self._open_reasoning_picker()
            return

        if command == "/tools":
            self._open_tools_picker()
            return

        if command == "/tool":
            if len(parts) == 1:
                self._open_tools_picker()
                return
            if len(parts) < 3:
                self._suspend()
                print("system> [tool] expected /tool <web_search|view_image> <on|off>")
                return
            tool_name = parts[1].lower()
            toggle = parts[2].lower()
            if tool_name not in {"web_search", "view_image"}:
                self._suspend()
                print("system> [tool] supported tools: web_search, view_image")
                return
            if toggle not in {"on", "off"}:
                self._suspend()
                print("system> [tool] toggle must be on or off")
                return
            enabled = toggle == "on"
            if tool_name == "view_image":
                self.current_tools["view_image"] = enabled
            else:
                self.current_tools["web_search"] = {} if enabled else None
            self._suspend()
            print(f"system> [tool] {tool_name} -> {toggle}")
            return

        self._suspend()
        print(f"system> unknown command: {text}")

    def _open_model_picker(self, *, force_refresh: bool) -> None:
        if force_refresh or self.model_cache is None:
            self.model_cache = self.session.list_models()

        models = self.model_cache
        options = [
            PickerOption(
                label="server default",
                value="__default__",
                description="use server-selected model",
            )
        ]
        for idx, model in enumerate(models, start=1):
            model_id = model.get("id", model.get("model", "?"))
            display_name = model.get("displayName", model_id)
            default_effort = model.get("defaultReasoningEffort", "?")
            current = "current" if model_id == self.current_model else ""
            options.append(
                PickerOption(
                    label=f"{display_name} [{model_id}]",
                    value=model_id,
                    description=f"default_effort={default_effort} {current}".rstrip(),
                )
            )
        selected = 0
        if self.current_model:
            for idx, option in enumerate(options):
                if option.value == self.current_model:
                    selected = idx
                    break
        self.panel = PickerState(
            kind="model",
            title="model picker",
            options=options,
            selected=selected,
            help_text="↑/↓ move • Enter select • Esc cancel",
        )

    def _open_reasoning_picker(self) -> None:
        options = [
            PickerOption(
                label="server default",
                value="__default__",
                description="use model/server default effort",
            )
        ]
        for effort in ReasoningEffort:
            current = "current" if effort.value == self.current_effort else ""
            options.append(PickerOption(label=effort.value, value=effort.value, description=current))
        selected = 0
        if self.current_effort:
            for idx, option in enumerate(options):
                if option.value == self.current_effort:
                    selected = idx
                    break
        self.panel = PickerState(
            kind="reasoning",
            title="reasoning picker",
            options=options,
            selected=selected,
            help_text="↑/↓ move • Enter select • Esc cancel",
        )

    def _open_tools_picker(self) -> None:
        web_search = "on" if self.current_tools.get("web_search") is not None else "off"
        view_image = "on" if self.current_tools.get("view_image") else "off"
        self.panel = PickerState(
            kind="tools",
            title="tool picker",
            options=[
                PickerOption("web_search", "web_search", web_search),
                PickerOption("view_image", "view_image", view_image),
            ],
            selected=0,
            help_text="↑/↓ move • Enter toggle • Esc close",
        )

    def _handle_panel_key(self, key: str) -> bool:
        assert self.panel is not None
        if key == "ESC":
            self.panel = None
            return True
        if key in {"UP", "k"}:
            self.panel.selected = max(0, self.panel.selected - 1)
            return True
        if key in {"DOWN", "j"}:
            self.panel.selected = min(len(self.panel.options) - 1, self.panel.selected + 1)
            return True
        if key in {"ENTER", " "}:
            self._apply_panel_selection()
            return True
        return False

    def _apply_panel_selection(self) -> None:
        assert self.panel is not None
        option = self.panel.options[self.panel.selected]
        self._suspend()

        if self.panel.kind == "model":
            if option.value == "__default__":
                self.current_model = None
                print("system> model reset to server default")
            else:
                self.current_model = option.value
                print(f"system> model set to {option.value}")
            self.panel = None
            return

        if self.panel.kind == "reasoning":
            if option.value == "__default__":
                self.current_effort = None
                print("system> reasoning reset to server default")
            else:
                self.current_effort = option.value
                print(f"system> reasoning set to {option.value}")
            self.panel = None
            return

        if self.panel.kind == "tools":
            if option.value == "view_image":
                self.current_tools["view_image"] = not bool(self.current_tools.get("view_image"))
            elif option.value == "web_search":
                self.current_tools["web_search"] = None if self.current_tools.get("web_search") is not None else {}
            print(f"system> tools: {self._tools_summary()}")
            self._open_tools_picker()
            return

    def _panel_lines(self) -> list[str]:
        if self.panel is None:
            return []
        lines = [f"[{self.panel.title}] {self.panel.help_text}"]
        for idx, option in enumerate(self.panel.options):
            marker = ">" if idx == self.panel.selected else " "
            label = option.label
            if option.description:
                label = label.ljust(42) + option.description
            lines.append(f"{marker} {label}")
        return lines

    def _tools_summary(self) -> str:
        web_search_enabled = self.current_tools.get("web_search") is not None
        view_image_enabled = bool(self.current_tools.get("view_image"))
        return f"web_search={'on' if web_search_enabled else 'off'}, view_image={'on' if view_image_enabled else 'off'}"

    def _tools_prompt_summary(self) -> str:
        web = "web" if self.current_tools.get("web_search") is not None else "no-web"
        img = "img" if self.current_tools.get("view_image") else "no-img"
        return f"{web}, {img}"

    def _turn_tools_payload(self) -> dict[str, Any]:
        return {
            "view_image": bool(self.current_tools.get("view_image")),
            "web_search": self.current_tools.get("web_search"),
        }


def render_entry(entry: TranscriptEntry, width: int) -> list[RenderLine]:
    entry_prefix = f"{entry.prefix}> "
    entry_prefix_attr = prefix_attr(entry.prefix)
    content_width = max(10, width - 1 - len(entry_prefix))
    text = entry.text or ""

    if entry.prefix.startswith("assistant"):
        return render_markdown_entry(entry_prefix, entry_prefix_attr, text, content_width)

    return render_plain_entry(entry_prefix, entry_prefix_attr, text, content_width)


def render_plain_entry(
    entry_prefix: str, entry_prefix_attr: int, text: str, content_width: int
) -> list[RenderLine]:
    lines: list[RenderLine] = []
    source_lines = text.splitlines() or [text]
    for source in source_lines:
        wrapped = textwrap.wrap(source, width=content_width) or [""]
        for idx, part in enumerate(wrapped):
            prefix = entry_prefix if idx == 0 else " " * len(entry_prefix)
            lines.append(
                RenderLine(
                    [
                        RenderSpan(prefix, entry_prefix_attr if idx == 0 else 0),
                        RenderSpan(part),
                    ]
                )
            )
    return lines


def render_markdown_entry(
    entry_prefix: str, entry_prefix_attr: int, text: str, content_width: int
) -> list[RenderLine]:
    lines: list[RenderLine] = []
    in_code_block = False
    normalized = normalize_markdown_text(text)

    for raw_line in normalized.splitlines() or [normalized]:
        stripped = raw_line.strip()
        base_attr = 0
        initial_indent = ""
        subsequent_indent = ""
        line_text = raw_line

        if stripped.startswith("```"):
            in_code_block = not in_code_block
            base_attr = curses.A_DIM
            line_text = stripped
        elif in_code_block:
            base_attr = curses.A_DIM
            line_text = raw_line
        else:
            heading = re.match(r"^(#{1,6})\s+(.*)$", stripped)
            bullet = re.match(r"^([*-])\s+(.*)$", stripped)
            numbered = re.match(r"^(\d+\.)\s+(.*)$", stripped)
            quote = re.match(r"^>\s?(.*)$", stripped)

            if heading:
                base_attr = curses.A_BOLD
                line_text = heading.group(2)
            elif bullet:
                initial_indent = f"{bullet.group(1)} "
                subsequent_indent = "  "
                line_text = bullet.group(2)
            elif numbered:
                initial_indent = f"{numbered.group(1)} "
                subsequent_indent = " " * len(initial_indent)
                line_text = numbered.group(2)
            elif quote:
                base_attr = curses.A_DIM
                initial_indent = "│ "
                subsequent_indent = "│ "
                line_text = quote.group(1)
            else:
                line_text = raw_line

        wrapped = textwrap.TextWrapper(
            width=content_width,
            initial_indent=initial_indent,
            subsequent_indent=subsequent_indent,
            break_long_words=False,
            drop_whitespace=False,
            replace_whitespace=False,
        ).wrap(line_text) or [initial_indent.rstrip()]

        for idx, part in enumerate(wrapped):
            prefix = entry_prefix if idx == 0 else " " * len(entry_prefix)
            spans = [RenderSpan(prefix, entry_prefix_attr if idx == 0 else 0)]
            spans.extend(parse_inline_markdown(part, base_attr))
            lines.append(RenderLine(spans))

    return lines


def normalize_markdown_text(text: str) -> str:
    text = text.replace("\r\n", "\n").replace("\r", "\n")
    text = re.sub(r"(?<!\n) {2,}([*-] |\d+\. |>\s)", r"\n\1", text)
    return text


def parse_inline_markdown(text: str, base_attr: int) -> list[RenderSpan]:
    spans: list[RenderSpan] = []
    for idx, part in enumerate(text.split("`")):
        if not part:
            continue
        attr = base_attr
        if idx % 2 == 1:
            attr |= curses.A_REVERSE
        spans.append(RenderSpan(part, attr))
    if not spans:
        spans.append(RenderSpan("", base_attr))
    return spans


def draw_render_line(stdscr, y: int, x: int, line: RenderLine, max_width: int) -> None:
    remaining = max_width
    col = x
    for span in line.spans:
        if remaining <= 0:
            break
        if not span.text:
            continue
        stdscr.addnstr(y, col, span.text, remaining, span.attr)
        used = min(len(span.text), remaining)
        col += used
        remaining -= used


def prefix_attr(prefix: str) -> int:
    if prefix.startswith("system"):
        return curses.A_DIM
    if prefix.startswith("assistant"):
        return curses.A_BOLD
    return 0


def format_args(arguments: Any) -> str:
    if not arguments:
        return "{}"
    try:
        return json.dumps(arguments, sort_keys=True)
    except TypeError:
        return str(arguments)


def extract_mcp_text(result: dict[str, Any]) -> str:
    content = result.get("content")
    if isinstance(content, list):
        parts: list[str] = []
        for item in content:
            if isinstance(item, dict) and item.get("type") == "text":
                parts.append(str(item.get("text", "")))
        return " | ".join(part for part in parts if part)
    return str(result)


def shorten(value: str | None, *, max_len: int = 12) -> str:
    if not value:
        return "(none)"
    if len(value) <= max_len:
        return value
    return value[:max_len]


def char_display_width(ch: str) -> int:
    if not ch:
        return 0
    if unicodedata.combining(ch):
        return 0
    if unicodedata.east_asian_width(ch) in {"W", "F"}:
        return 2
    return 1


def display_width(text: str) -> int:
    return sum(char_display_width(ch) for ch in text)


def clip_display_text(text: str, max_width: int) -> str:
    if max_width <= 0:
        return ""
    width = 0
    parts: list[str] = []
    for ch in text:
        ch_width = char_display_width(ch)
        if width + ch_width > max_width:
            break
        parts.append(ch)
        width += ch_width
    return "".join(parts)


def fit_input_to_width(text: str, cursor_pos: int, width: int) -> tuple[str, int]:
    if width <= 0:
        return "", 0
    if display_width(text) <= width:
        return text, display_width(text[:cursor_pos])

    start = 0
    end = len(text)
    while start < cursor_pos and display_width(text[start:cursor_pos]) >= width:
        start += 1

    prefix = "…" if start > 0 else ""
    suffix = ""
    available = width - display_width(prefix)
    while end > start and display_width(text[start:end]) > available:
        end -= 1
    if end < len(text):
        suffix = "…"
        available = width - display_width(prefix) - display_width(suffix)
        while end > start and display_width(text[start:end]) > available:
            end -= 1

    visible = prefix + text[start:end] + suffix
    cursor_cols = display_width(prefix) + display_width(text[start:cursor_pos])
    cursor_cols = min(cursor_cols, display_width(visible))
    return visible, cursor_cols


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Attach a small TUI to the live Cilux guest Codex app-server.")
    parser.add_argument("--ws-url", default=os.environ.get("CILUX_WS_URL", DEFAULT_WS_URL))
    parser.add_argument(
        "--token-file",
        default=os.environ.get("CILUX_WS_TOKEN_FILE", str(DEFAULT_TOKEN_PATH)),
        help="Path to the guest websocket capability token",
    )
    parser.add_argument("--cwd", default=DEFAULT_CWD)
    parser.add_argument("--approval-policy", default="never")
    parser.add_argument("--sandbox", default="danger-full-access")
    parser.add_argument("--personality", default="friendly")
    parser.add_argument("--model", default=None)
    parser.add_argument(
        "--fullscreen",
        action="store_true",
        help="Use the fullscreen curses UI instead of the shell-native scrollback UI.",
    )
    parser.add_argument(
        "--prompt",
        default=None,
        help="Run one prompt in non-interactive mode and print the streamed transcript",
    )
    return parser.parse_args()


def run_prompt_mode(session: AppServerSession, prompt: str) -> int:
    printer = ShellTurnPrinter()
    printed_any_assistant = False

    def on_event(kind: str, payload: Any) -> None:
        nonlocal printed_any_assistant
        if kind == "assistant_start":
            printed_any_assistant = True
            printer.assistant_start(str(payload))
        elif kind == "assistant_delta":
            printer.assistant_delta(str(payload))
        elif kind == "assistant_end":
            printer.assistant_end()
        elif kind == "system":
            printer.system(str(payload))
        elif kind == "turn_completed":
            printer.assistant_end()

    print(f"you> {prompt}")
    session.turn(prompt, on_event)
    if not printed_any_assistant:
        print("assistant> [no text]")
    return 0


def main() -> int:
    args = parse_args()
    token_path = pathlib.Path(args.token_file)
    if not token_path.exists():
        print(f"token file not found: {token_path}", file=sys.stderr)
        return 1

    token = token_path.read_text(encoding="utf-8").strip()
    if not token:
        print(f"token file is empty: {token_path}", file=sys.stderr)
        return 1

    try:
        with AppServerSession(
            args.ws_url,
            token,
            cwd=args.cwd,
            approval_policy=args.approval_policy,
            sandbox=args.sandbox,
            personality=args.personality,
            model=args.model,
        ) as session:
            if args.prompt:
                return run_prompt_mode(session, args.prompt)

            if args.fullscreen:
                app = TuiApp(session)
                curses.wrapper(app.run)
            else:
                app = ShellApp(session)
                app.run()
    except KeyboardInterrupt:
        print("\nInterrupted.", file=sys.stderr)
        return 130
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
