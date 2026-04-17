use crate::{catalog, resources, tools};
use anyhow::{Context, Result};
use serde_json::{json, Value};
use std::io::{BufRead, Write};
use std::path::Path;

pub(crate) fn serve(socket: &Path, mut stdin: impl BufRead, mut stdout: impl Write) -> Result<()> {
    let mut line = String::new();
    loop {
        line.clear();
        let read = stdin.read_line(&mut line).context("read MCP request")?;
        if read == 0 {
            break;
        }
        if line.trim().is_empty() {
            continue;
        }

        let message: Value = serde_json::from_str(&line).context("parse JSON-RPC request")?;
        if message.get("id").is_none() {
            continue;
        }

        let id = message.get("id").cloned().unwrap_or(Value::Null);
        let method = message
            .get("method")
            .and_then(Value::as_str)
            .unwrap_or_default();
        let params = message.get("params").cloned().unwrap_or_else(|| json!({}));
        let capabilities = catalog::CatalogCapabilities::detect(socket);

        let response = match method {
            "initialize" => initialize_response(id, &params),
            "ping" => success(id, json!({})),
            "tools/list" => success(id, catalog::list_tools(&capabilities)),
            "tools/call" => tools::tool_call_response(socket, id, &params),
            "resources/list" => success(id, catalog::list_resources(&capabilities)),
            "resources/read" => resources::resource_read_response(socket, id, &params),
            "resources/templates/list" => {
                success(id, catalog::list_resource_templates(&capabilities))
            }
            _ => error(id, -32601, format!("method not found: {method}")),
        };

        serde_json::to_writer(&mut stdout, &response).context("write JSON-RPC response")?;
        stdout.write_all(b"\n").context("write response newline")?;
        stdout.flush().context("flush stdout")?;
    }

    Ok(())
}

fn initialize_response(id: Value, params: &Value) -> Value {
    let protocol_version = params
        .get("protocolVersion")
        .cloned()
        .unwrap_or_else(|| json!("2025-06-18"));
    success(
        id,
        json!({
            "protocolVersion": protocol_version,
            "serverInfo": {
                "name": "cilux-mcp",
                "title": "Cilux Broker MCP",
                "version": env!("CARGO_PKG_VERSION"),
            },
            "capabilities": {
                "tools": { "listChanged": false },
                "resources": { "listChanged": false, "subscribe": false },
            }
        }),
    )
}

pub(crate) fn success(id: Value, result: Value) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "result": result })
}

pub(crate) fn error(id: Value, code: i32, message: String) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "error": { "code": code, "message": message } })
}
