#!/usr/bin/env python3
"""
Cilux TUI Client - A minimal terminal UI for interacting with Cilux MCP server.
Connects to ws://127.0.0.1:18765 and provides an interactive chat interface.
"""

import asyncio
import json
import sys
from datetime import datetime
from typing import Optional

try:
    import websockets
    from textual.app import App, ComposeResult
    from textual.containers import Horizontal, Vertical, VerticalScroll
    from textual.reactive import reactive
    from textual.widgets import (
        Button,
        Header,
        Footer,
        Input,
        Label,
        Static,
    )
except ImportError as e:
    print(f"Error: Missing dependency - {e}")
    print("Please install: pip install textual websockets")
    sys.exit(1)


class MessageDisplay(Static):
    """Widget to display a single message."""
    
    def __init__(self, sender: str, content: str, timestamp: str = "", is_user: bool = False):
        self.sender = sender
        self.content = content
        self.timestamp = timestamp or datetime.now().strftime("%H:%M:%S")
        self.is_user = is_user
        super().__init__()
    
    def compose(self) -> ComposeResult:
        with Vertical(classes="message-container user" if self.is_user else "message-container"):
            with Horizontal(classes="message-header"):
                yield Label(f"[{self.timestamp}] {self.sender}:", classes="message-sender user" if self.is_user else "message-sender")
            yield Label(self.content, classes="message-content")


class CiluxTUI(App):
    """Main TUI application for Cilux MCP client."""
    
    CSS = """
    Screen {
        align: center middle;
    }
    
    #main-container {
        width: 100%;
        height: 100%;
    }
    
    #messages-scroll {
        width: 100%;
        height: 1fr;
        border: solid $primary;
        padding: 1;
    }
    
    #input-area {
        width: 100%;
        height: auto;
        padding: 1;
    }
    
    #message-input {
        width: 1fr;
    }
    
    #send-button {
        width: auto;
        margin-left: 1;
    }
    
    #status-bar {
        width: 100%;
        height: auto;
        background: $surface;
        color: $text;
        padding: 0 1;
        border-top: solid $primary;
    }
    
    .connected {
        color: green;
    }
    
    .disconnected {
        color: red;
    }
    
    .connecting {
        color: yellow;
    }
    
    .message-container {
        margin: 1 0;
        padding: 1;
        background: $surface-darken-1;
    }
    
    .message-container.user {
        background: $primary-darken-2;
    }
    
    .message-header {
        height: auto;
    }
    
    .message-sender {
        color: cyan;
        text-style: bold;
    }
    
    .message-sender.user {
        color: yellow;
    }
    
    .message-content {
        margin-left: 2;
    }
    
    .title {
        text-align: center;
        text-style: bold;
        color: $primary;
    }
    
    .subtitle {
        text-align: center;
        color: $text-muted;
    }
    """
    
    connection_status = reactive("disconnected")
    websocket: Optional[websockets.WebSocketClientProtocol] = None
    uri = "ws://127.0.0.1:18765"
    
    def compose(self) -> ComposeResult:
        yield Header(show_clock=True)
        
        with Vertical(id="main-container"):
            with VerticalScroll(id="messages-scroll"):
                yield Label("Welcome to Cilux TUI Client", classes="title")
                yield Label(f"Server: {self.uri}", classes="subtitle")
            
            with Horizontal(id="status-bar"):
                yield Label("Status: ", id="status-label")
                yield Label("Disconnected", id="status-value", classes="disconnected")
            
            with Horizontal(id="input-area"):
                yield Input(placeholder="Type your message here...", id="message-input")
                yield Button("Send", id="send-button", variant="primary")
        
        yield Footer()
    
    async def on_mount(self) -> None:
        """Called when the app is mounted."""
        self.messages_scroll = self.query_one("#messages-scroll", VerticalScroll)
        self.status_value = self.query_one("#status-value", Label)
        self.message_input = self.query_one("#message-input", Input)
        
        # Start connection
        asyncio.create_task(self.connect_websocket())
    
    def watch_connection_status(self, status: str) -> None:
        """Watch for connection status changes."""
        if hasattr(self, 'status_value'):
            self.status_value.update(status.title())
            self.status_value.classes = status
    
    async def connect_websocket(self) -> None:
        """Establish WebSocket connection."""
        self.connection_status = "connecting"
        
        while True:
            try:
                async with websockets.connect(self.uri) as websocket:
                    self.websocket = websocket
                    self.connection_status = "connected"
                    self.add_message("System", f"Connected to {self.uri}")
                    
                    # Listen for messages
                    async for message in websocket:
                        await self.handle_message(message)
                        
            except websockets.exceptions.ConnectionClosed:
                self.connection_status = "disconnected"
                self.add_message("System", "Connection closed. Retrying in 5 seconds...")
            except Exception as e:
                self.connection_status = "disconnected"
                self.add_message("System", f"Error: {e}. Retrying in 5 seconds...")
            
            self.websocket = None
            await asyncio.sleep(5)
    
    async def handle_message(self, message: str) -> None:
        """Handle incoming WebSocket message."""
        try:
            # Try to parse as JSON
            data = json.loads(message)
            content = json.dumps(data, indent=2)
        except json.JSONDecodeError:
            # Treat as plain text
            content = message
        
        self.add_message("Server", content)
    
    def add_message(self, sender: str, content: str, is_user: bool = False) -> None:
        """Add a message to the display."""
        timestamp = datetime.now().strftime("%H:%M:%S")
        message_widget = MessageDisplay(sender, content, timestamp, is_user)
        self.messages_scroll.mount(message_widget)
        self.messages_scroll.scroll_end(animate=False)
    
    async def send_message(self, text: str) -> None:
        """Send a message to the server."""
        if not self.websocket:
            self.add_message("System", "Not connected to server")
            return
        
        try:
            # Send as JSON if it looks like JSON, otherwise plain text
            try:
                data = json.loads(text)
                await self.websocket.send(json.dumps(data))
            except json.JSONDecodeError:
                await self.websocket.send(text)
            
            self.add_message("You", text, is_user=True)
        except Exception as e:
            self.add_message("System", f"Failed to send: {e}")
    
    async def on_button_pressed(self, event: Button.Pressed) -> None:
        """Handle button press."""
        if event.button.id == "send-button":
            await self.handle_send()
    
    async def on_input_submitted(self, event: Input.Submitted) -> None:
        """Handle input submission."""
        if event.input.id == "message-input":
            await self.handle_send()
    
    async def handle_send(self) -> None:
        """Handle sending a message."""
        text = self.message_input.value.strip()
        if text:
            await self.send_message(text)
            self.message_input.value = ""


def main():
    """Entry point."""
    print("Starting Cilux TUI Client...")
    print(f"Connecting to: ws://127.0.0.1:18765")
    print("\nRequired dependencies:")
    print("  pip install textual websockets")
    print("\nPress Ctrl+C to exit")
    print("-" * 50)
    
    app = CiluxTUI()
    app.run()


if __name__ == "__main__":
    main()
