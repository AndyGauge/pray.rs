mod store;
mod tools;

use serde_json::{json, Value};
use std::io::{self, BufRead, Write};

use store::ProjectStore;

fn main() {
    let workspace = std::env::var("WORKSPACE_DIR")
        .unwrap_or_else(|_| ".workspace".to_string());

    let store = ProjectStore::new(workspace);
    let stdin  = io::stdin();
    let stdout = io::stdout();
    let mut out = io::BufWriter::new(stdout.lock());

    for line in stdin.lock().lines() {
        let line = match line {
            Ok(l) if !l.trim().is_empty() => l,
            _ => continue,
        };

        let request: Value = match serde_json::from_str(&line) {
            Ok(v) => v,
            Err(e) => {
                let resp = json!({ "jsonrpc": "2.0", "error": { "code": -32700, "message": e.to_string() }, "id": null });
                writeln!(out, "{resp}").ok();
                out.flush().ok();
                continue;
            }
        };

        let id     = request.get("id").cloned().unwrap_or(Value::Null);
        let method = request["method"].as_str().unwrap_or("");
        let params = request.get("params").cloned().unwrap_or(json!({}));

        let result = match method {
            "initialize" => json!({
                "protocolVersion": "2024-11-05",
                "capabilities": { "tools": {} },
                "serverInfo": { "name": "thanksgivings-project-mcp", "version": "0.1.0" }
            }),
            "tools/list" => tools::list_tools(),
            "tools/call" => {
                let name = params["name"].as_str().unwrap_or("");
                let args = params.get("arguments").cloned().unwrap_or(json!({}));
                tools::call_tool(&store, name, &args)
            }
            "notifications/initialized" | "ping" => json!({}),
            _ => {
                let err = json!({ "jsonrpc": "2.0", "error": { "code": -32601, "message": "method not found" }, "id": id });
                writeln!(out, "{err}").ok();
                out.flush().ok();
                continue;
            }
        };

        let resp = json!({ "jsonrpc": "2.0", "result": result, "id": id });
        writeln!(out, "{resp}").ok();
        out.flush().ok();
    }
}
