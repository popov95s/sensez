from __future__ import annotations

import json
import subprocess
from pathlib import Path

JsonObject = dict[str, object]


class McpClient:
    def __init__(self, sensez: Path):
        self.proc = subprocess.Popen(
            [str(sensez), "mcp", "serve"],
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
            stderr = self._stderr_text()
            message = f"MCP server exited {self.proc.returncode}"
            if stderr:
                message = f"{message}: {stderr}"
            raise RuntimeError(message)

    def request(self, method: str, params: JsonObject | None = None) -> JsonObject:
        msg: JsonObject = {
            "jsonrpc": "2.0",
            "id": self.next_id,
            "method": method,
        }
        self.next_id += 1
        if params is not None:
            msg["params"] = params
        return self._send(msg)

    def call_tool(self, name: str, arguments: JsonObject) -> JsonObject:
        return self.request(
            "tools/call",
            {"name": name, "arguments": arguments},
        )["result"]

    def _send(self, msg: JsonObject) -> JsonObject:
        if not self.proc.stdin or not self.proc.stdout:
            raise RuntimeError("MCP process pipes are closed")
        self.proc.stdin.write(json.dumps(msg) + "\n")
        self.proc.stdin.flush()
        line = self.proc.stdout.readline()
        if not line:
            stderr = self._stderr_text()
            message = "MCP server closed stdout"
            if stderr:
                message = f"{message}: {stderr}"
            raise RuntimeError(message)
        resp = json.loads(line)
        if not isinstance(resp, dict):
            raise RuntimeError("MCP server returned a non-object response")
        if "error" in resp:
            raise RuntimeError(str(resp["error"]))
        return resp

    def _stderr_text(self) -> str | None:
        if self.proc.stderr is None:
            return None
        return self.proc.stderr.read()


def text_json(result: JsonObject) -> object:
    text = result["content"][0]["text"]
    return json.loads(text)
