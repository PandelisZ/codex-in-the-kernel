#!/usr/bin/env python3
import asyncio
import json
import os
import pathlib
import subprocess
import sys
import venv
from typing import Any


ROOT = pathlib.Path(__file__).resolve().parents[1]
TOKEN_PATH = pathlib.Path(
    os.environ.get(
        "CILUX_WS_TOKEN_FILE",
        str(ROOT / "artifacts" / "rootfs" / "etc" / "cilux" / "ws-token"),
    )
)
WS_URL = os.environ.get("CILUX_WS_URL", "ws://127.0.0.1:8765")


def ensure_websockets():
    try:
        import websockets  # type: ignore
    except ImportError:
        venv_dir = ROOT / "tests" / ".venv"
        if not venv_dir.exists():
            venv.EnvBuilder(with_pip=True).create(venv_dir)
        venv_python = venv_dir / "bin" / "python3"
        subprocess.check_call([str(venv_python), "-m", "pip", "install", "websockets"])
        os.execv(str(venv_python), [str(venv_python), str(pathlib.Path(__file__).resolve())])
    return websockets


websockets = ensure_websockets()

class AppServerClient:
    def __init__(self, ws):
        self.ws = ws
        self.next_id = 1
        self.pending: list[dict[str, Any]] = []

    async def request(self, method: str, params: dict[str, Any]) -> dict[str, Any]:
        request_id = self.next_id
        self.next_id += 1
        await self.ws.send(json.dumps({"id": request_id, "method": method, "params": params}))

        while True:
            message = json.loads(await self.ws.recv())
            if message.get("id") == request_id:
                if "error" in message:
                    raise RuntimeError(f"{method} failed: {message['error']}")
                return message["result"]
            self.pending.append(message)

    async def notify(self, method: str, params: dict[str, Any]) -> None:
        await self.ws.send(json.dumps({"method": method, "params": params}))

    async def stream_turn(self, turn_id: str) -> tuple[str, list[dict[str, Any]], list[str]]:
        message_text: list[str] = []
        notifications: list[dict[str, Any]] = []
        tools: list[str] = []

        while True:
            if self.pending:
                message = self.pending.pop(0)
            else:
                message = json.loads(await self.ws.recv())

            if "method" not in message:
                continue

            notifications.append(message)
            method = message["method"]
            params = message.get("params", {})
            if method == "item/agentMessage/delta":
                if params.get("turnId") == turn_id:
                    message_text.append(params.get("delta", ""))
            elif method in {"item/started", "item/completed"}:
                item = params.get("item", {})
                if item.get("type") == "mcpToolCall":
                    tool = item.get("tool")
                    if tool:
                        tools.append(tool)
            elif method == "turn/completed" and params.get("turn", {}).get("id") == turn_id:
                return "".join(message_text), notifications, tools


async def main() -> None:
    token = TOKEN_PATH.read_text(encoding="utf-8").strip()
    headers = {"Authorization": f"Bearer {token}"}

    connect_kwargs = {"max_size": None}
    try:
        websocket_cm = websockets.connect(WS_URL, additional_headers=headers, **connect_kwargs)
    except TypeError:
        websocket_cm = websockets.connect(WS_URL, extra_headers=headers, **connect_kwargs)

    async with websocket_cm as ws:
        client = AppServerClient(ws)
        await client.request(
            "initialize",
            {
                "clientInfo": {
                    "name": "cilux_e2e",
                    "title": "Cilux E2E",
                    "version": "0.1.0",
                }
            },
        )
        await client.notify("initialized", {})

        thread = await client.request(
            "thread/start",
            {
                "cwd": "/workspace",
                "approvalPolicy": "never",
                "sandbox": "danger-full-access",
                "personality": "friendly",
            },
        )
        thread_id = thread["thread"]["id"]

        positive_turn = await client.request(
            "turn/start",
            {
                "threadId": thread_id,
                "input": [
                    {
                        "type": "text",
                        "text": "Use the cilux_kernel_snapshot tool and the cilux_events_tail tool with limit 16. Summarize the current trace_mask, event_count, and the two most recent event kinds in exactly three bullet lines.",
                        "textElements": [],
                    }
                ],
            },
        )
        positive_text, _, positive_tools = await client.stream_turn(positive_turn["turn"]["id"])

        if "cilux_kernel_snapshot" not in positive_tools or "cilux_events_tail" not in positive_tools:
            raise RuntimeError(f"expected MCP kernel tools, saw {positive_tools}")
        lowered_positive = positive_text.lower()
        if "trace" not in lowered_positive or "event" not in lowered_positive:
            raise RuntimeError(f"unexpected positive turn output: {positive_text}")

        negative_turn = await client.request(
            "turn/start",
            {
                "threadId": thread_id,
                "input": [
                    {
                        "type": "text",
                        "text": "Call the cilux_trace_configure tool with trace_mask 4294967295 and report the exact failure in one sentence.",
                        "textElements": [],
                    }
                ],
            },
        )
        negative_text, _, negative_tools = await client.stream_turn(negative_turn["turn"]["id"])

        if "cilux_trace_configure" not in negative_tools:
            raise RuntimeError(f"expected trace configure tool call, saw {negative_tools}")
        lowered = negative_text.lower()
        if "error" not in lowered and "invalid" not in lowered and "failed" not in lowered:
            raise RuntimeError(f"negative turn did not surface an error: {negative_text}")

        health_turn = await client.request(
            "turn/start",
            {
                "threadId": thread_id,
                "input": [
                    {
                        "type": "text",
                        "text": "Use the cilux_health tool and the cilux_system_read tool with selector proc_modules. Report in exactly three bullet lines whether debugfs_ready is true, whether netlink_ready is true, and whether rust_cilux appears in /proc/modules.",
                        "textElements": [],
                    }
                ],
            },
        )
        health_text, _, health_tools = await client.stream_turn(health_turn["turn"]["id"])

        if "cilux_health" not in health_tools or "cilux_system_read" not in health_tools:
            raise RuntimeError(f"expected health and system read tool calls, saw {health_tools}")
        lowered_health = health_text.lower()
        if "debugfs" not in lowered_health or "netlink" not in lowered_health or "rust_cilux" not in lowered_health:
            raise RuntimeError(f"unexpected health/system turn output: {health_text}")

        print("positive turn ok")
        print("negative turn ok")
        print("health turn ok")


if __name__ == "__main__":
    asyncio.run(main())
