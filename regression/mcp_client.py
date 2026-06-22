from __future__ import annotations

import json
import subprocess
from pathlib import Path
from typing import Any


class McpClient:
    def __init__(self, sense: Path):
        self.proc = subprocess.Popen(
            [str(sense), "mcp", "serve"],
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
        )
        self.next_id = 1

    def close(self) -> None:
        if self.proc.stdin:
            self.proc.stdin.close()
        self.proc.wait(timeout=30)
        if self.proc.returncode != 0:
            stderr = self.proc.stderr.read() if self.proc.stderr else ""
            raise RuntimeError(f"MCP server exited {self.proc.returncode}: {stderr}")

    def request(self, method: str, params: dict[str, Any] | None = None) -> Any:
        msg: dict[str, Any] = {
            "jsonrpc": "2.0",
            "id": self.next_id,
            "method": method,
        }
        self.next_id += 1
        if params is not None:
            msg["params"] = params
        return self._send(msg)

    def call_tool(self, name: str, arguments: dict[str, Any]) -> Any:
        return self.request(
            "tools/call",
            {"name": name, "arguments": arguments},
        )["result"]

    def _send(self, msg: dict[str, Any]) -> Any:
        if not self.proc.stdin or not self.proc.stdout:
            raise RuntimeError("MCP process pipes are closed")
        self.proc.stdin.write(json.dumps(msg) + "\n")
        self.proc.stdin.flush()
        line = self.proc.stdout.readline()
        if not line:
            stderr = self.proc.stderr.read() if self.proc.stderr else ""
            raise RuntimeError(f"MCP server closed stdout: {stderr}")
        resp = json.loads(line)
        if "error" in resp:
            raise RuntimeError(resp["error"])
        return resp


def text_json(result: Any) -> Any:
    text = result["content"][0]["text"]
    return json.loads(text)
