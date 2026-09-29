#!/usr/bin/env python3
"""Fetch every public prayer from pray.rs over MCP, for the website build.

Speaks MCP (JSON-RPC over HTTP) to the app's /mcp endpoint as an anonymous
client — which can only call `list_public_prayers` — and pages through the
tool's `structuredContent` until `next_offset` is null. Writes
website/data/public_prayers.json, which Hugo reads as `site.Data.public_prayers`.

The output is gitignored on purpose: the collection is rebuilt from the live
app on every site build, so a prayer its author makes private or deletes
disappears at the next build instead of living on in git history.

    python3 website/scripts/fetch_public_prayers.py            # https://pray.rs/mcp
    MCP_URL=http://localhost:3000/mcp python3 website/scripts/fetch_public_prayers.py
"""

import datetime
import json
import os
import pathlib
import sys
import urllib.request

MCP_URL = os.environ.get("MCP_URL", "https://pray.rs/mcp")
OUT = pathlib.Path(__file__).resolve().parent.parent / "data" / "public_prayers.json"
PAGE_SIZE = 100


class McpClient:
    def __init__(self, url):
        self.url = url
        self.next_id = 0

    def _post(self, payload):
        req = urllib.request.Request(
            self.url,
            data=json.dumps(payload).encode(),
            headers={
                "Content-Type": "application/json",
                "Accept": "application/json, text/event-stream",
                "User-Agent": "pray.rs-website-build",
            },
        )
        with urllib.request.urlopen(req, timeout=30) as resp:
            body = resp.read()
        return json.loads(body) if body else None

    def request(self, method, params=None):
        self.next_id += 1
        reply = self._post({"jsonrpc": "2.0", "id": self.next_id, "method": method, "params": params or {}})
        if "error" in reply:
            raise RuntimeError(f"{method} failed: {reply['error']}")
        return reply["result"]

    def notify(self, method):
        self._post({"jsonrpc": "2.0", "method": method})

    def call_tool(self, name, arguments):
        result = self.request("tools/call", {"name": name, "arguments": arguments})
        if result.get("isError"):
            text = " ".join(c.get("text", "") for c in result.get("content", []))
            raise RuntimeError(f"{name} returned an error: {text}")
        return result


def main():
    client = McpClient(MCP_URL)
    info = client.request("initialize", {
        "protocolVersion": "2024-11-05",
        "capabilities": {},
        "clientInfo": {"name": "pray.rs-website", "version": "1"},
    })
    client.notify("notifications/initialized")

    tools = {t["name"] for t in client.request("tools/list")["tools"]}
    if "list_public_prayers" not in tools:
        sys.exit(f"{MCP_URL} doesn't offer list_public_prayers (tools: {sorted(tools)})")

    prayers, offset = [], 0
    while offset is not None:
        result = client.call_tool("list_public_prayers", {"limit": PAGE_SIZE, "offset": offset})
        page = result.get("structuredContent")
        if page is None:
            sys.exit(f"{MCP_URL} returned no structuredContent; the server needs the "
                     "list_public_prayers structuredContent change deployed")
        prayers.extend(page["prayers"])
        offset = page["next_offset"]

    OUT.parent.mkdir(parents=True, exist_ok=True)
    OUT.write_text(json.dumps({
        "source": MCP_URL,
        "server": info.get("serverInfo", {}),
        "fetched_at": datetime.datetime.now(datetime.timezone.utc).isoformat(timespec="seconds"),
        "prayers": prayers,
    }, indent=2, ensure_ascii=False) + "\n")
    print(f"fetched {len(prayers)} public prayers from {MCP_URL} -> {OUT}")


if __name__ == "__main__":
    main()
