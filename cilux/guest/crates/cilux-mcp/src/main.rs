use anyhow::{Context, Result};
use cilux_common::{
    call_broker, pretty_json, BrokerRequest, BufferClearRequest, HealthRequest,
    KernelEventsTailRequest, KernelSnapshotRequest, SystemReadRequest, SystemReadSelector,
    TraceConfigureRequest, DEFAULT_BROKER_SOCKET,
};
use clap::Parser;
use serde_json::{json, Value};
use std::io::{self, BufRead, Write};
use std::path::PathBuf;

#[derive(Parser, Debug)]
struct Args {
    #[arg(long, default_value = DEFAULT_BROKER_SOCKET)]
    socket: PathBuf,
}

fn main() -> Result<()> {
    let args = Args::parse();
    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut stdout = stdout.lock();

    for line in stdin.lock().lines() {
        let line = line.context("read MCP request")?;
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

        let response = match method {
            "initialize" => initialize_response(id, &params),
            "ping" => success(id, json!({})),
            "tools/list" => success(id, list_tools()),
            "tools/call" => tool_call_response(&args.socket, id, &params),
            "resources/list" => success(id, list_resources()),
            "resources/read" => resource_read_response(&args.socket, id, &params),
            "resources/templates/list" => success(id, list_resource_templates()),
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

fn list_tools() -> Value {
    json!({
        "tools": [
            {
                "name": "cilux_kernel_snapshot",
                "description": "Read the latest kernel capability and state snapshot from the Cilux broker.",
                "inputSchema": { "type": "object", "properties": {} }
            },
            {
                "name": "cilux_events_tail",
                "description": "Read the most recent kernel event records from the Cilux ring buffer.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "limit": { "type": "integer", "minimum": 1, "maximum": 256 }
                    }
                }
            },
            {
                "name": "cilux_trace_configure",
                "description": "Set the active Cilux kernel trace mask using the broker's constrained control path.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "trace_mask": { "type": "integer", "minimum": 0 }
                    },
                    "required": ["trace_mask"]
                }
            },
            {
                "name": "cilux_health",
                "description": "Read broker health, debugfs readiness, and Generic Netlink reachability.",
                "inputSchema": { "type": "object", "properties": {} }
            },
            {
                "name": "cilux_buffer_clear",
                "description": "Clear the Cilux kernel event ring buffer through the constrained broker path.",
                "inputSchema": { "type": "object", "properties": {} }
            },
            {
                "name": "cilux_system_read",
                "description": "Read curated kernel-adjacent guest state such as dmesg and selected /proc snapshots.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "selector": {
                            "type": "string",
                            "enum": [
                                "dmesg",
                                "proc_modules",
                                "proc_meminfo",
                                "proc_loadavg",
                                "proc_uptime",
                                "proc_cpuinfo",
                                "proc_interrupts",
                                "proc_vmstat",
                                "proc_buddyinfo",
                                "proc_zoneinfo"
                            ]
                        }
                    },
                    "required": ["selector"]
                }
            }
        ]
    })
}

fn list_resources() -> Value {
    json!({
        "resources": [
            {
                "uri": "cilux://caps",
                "name": "Cilux Capabilities",
                "description": "Current kernel-side capability map for the Cilux harness.",
                "mimeType": "application/json"
            },
            {
                "uri": "cilux://state",
                "name": "Cilux State",
                "description": "Current Cilux trace mask, counters, and ring-buffer state.",
                "mimeType": "application/json"
            },
            {
                "uri": "cilux://events",
                "name": "Cilux Events",
                "description": "Recent Cilux kernel events as newline-delimited JSON.",
                "mimeType": "application/x-ndjson"
            },
            {
                "uri": "cilux://health",
                "name": "Cilux Health",
                "description": "Broker health and guest kernel integration readiness.",
                "mimeType": "application/json"
            },
            {
                "uri": "cilux://system/dmesg",
                "name": "Guest Dmesg",
                "description": "Kernel ring buffer via the broker's curated system read path.",
                "mimeType": "text/plain"
            },
            {
                "uri": "cilux://system/proc_modules",
                "name": "Guest Proc Modules",
                "description": "Current loaded kernel modules from /proc/modules.",
                "mimeType": "text/plain"
            },
            {
                "uri": "cilux://system/proc_meminfo",
                "name": "Guest Proc Meminfo",
                "description": "Current memory accounting from /proc/meminfo.",
                "mimeType": "text/plain"
            },
            {
                "uri": "cilux://system/proc_loadavg",
                "name": "Guest Proc Loadavg",
                "description": "Current scheduler load from /proc/loadavg.",
                "mimeType": "text/plain"
            },
            {
                "uri": "cilux://system/proc_uptime",
                "name": "Guest Proc Uptime",
                "description": "Current uptime from /proc/uptime.",
                "mimeType": "text/plain"
            },
            {
                "uri": "cilux://system/proc_cpuinfo",
                "name": "Guest Proc Cpuinfo",
                "description": "Current CPU topology and features from /proc/cpuinfo.",
                "mimeType": "text/plain"
            },
            {
                "uri": "cilux://system/proc_interrupts",
                "name": "Guest Proc Interrupts",
                "description": "Current interrupt counters from /proc/interrupts.",
                "mimeType": "text/plain"
            },
            {
                "uri": "cilux://system/proc_vmstat",
                "name": "Guest Proc Vmstat",
                "description": "Current virtual-memory counters from /proc/vmstat.",
                "mimeType": "text/plain"
            },
            {
                "uri": "cilux://system/proc_buddyinfo",
                "name": "Guest Proc Buddyinfo",
                "description": "Current page allocator free-area state from /proc/buddyinfo.",
                "mimeType": "text/plain"
            },
            {
                "uri": "cilux://system/proc_zoneinfo",
                "name": "Guest Proc Zoneinfo",
                "description": "Current zone watermarks and page accounting from /proc/zoneinfo.",
                "mimeType": "text/plain"
            }
        ]
    })
}

fn list_resource_templates() -> Value {
    json!({
        "resourceTemplates": [
            {
                "uriTemplate": "cilux://events/{limit}",
                "name": "Cilux Events Tail",
                "description": "Recent Cilux kernel events with a caller-selected limit.",
                "mimeType": "application/x-ndjson"
            },
            {
                "uriTemplate": "cilux://system/{selector}",
                "name": "Cilux System Snapshot",
                "description": "Curated kernel-adjacent guest state selected by name.",
                "mimeType": "text/plain"
            }
        ]
    })
}

fn tool_call_response(socket: &PathBuf, id: Value, params: &Value) -> Value {
    let name = params
        .get("name")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let arguments = params
        .get("arguments")
        .cloned()
        .unwrap_or_else(|| json!({}));

    let result = match name {
        "cilux_kernel_snapshot" => call_broker(
            socket,
            &BrokerRequest::KernelSnapshot(KernelSnapshotRequest::default()),
        )
        .and_then(|response| response.into_result())
        .and_then(|result| pretty_json(&result)),
        "cilux_events_tail" => {
            let limit = arguments.get("limit").and_then(Value::as_u64).unwrap_or(32) as usize;
            call_broker(
                socket,
                &BrokerRequest::KernelEventsTail(KernelEventsTailRequest { limit }),
            )
            .and_then(|response| response.into_result())
            .and_then(|result| pretty_json(&result))
        }
        "cilux_trace_configure" => {
            let trace_mask = arguments
                .get("trace_mask")
                .and_then(Value::as_u64)
                .unwrap_or_default() as u32;
            call_broker(
                socket,
                &BrokerRequest::TraceConfigure(TraceConfigureRequest { trace_mask }),
            )
            .and_then(|response| response.into_result())
            .and_then(|result| pretty_json(&result))
        }
        "cilux_health" => call_broker(socket, &BrokerRequest::Health(HealthRequest::default()))
            .and_then(|response| response.into_result())
            .and_then(|result| pretty_json(&result)),
        "cilux_buffer_clear" => call_broker(
            socket,
            &BrokerRequest::BufferClear(BufferClearRequest::default()),
        )
        .and_then(|response| response.into_result())
        .and_then(|result| pretty_json(&result)),
        "cilux_system_read" => arguments
            .get("selector")
            .and_then(Value::as_str)
            .ok_or_else(|| anyhow::anyhow!("missing selector"))
            .and_then(parse_system_selector)
            .and_then(|selector| {
                call_broker(
                    socket,
                    &BrokerRequest::SystemRead(SystemReadRequest { selector }),
                )
                .and_then(|response| response.into_result())
                .and_then(|result| pretty_json(&result))
            }),
        _ => Err(anyhow::anyhow!("unknown tool `{name}`")),
    };

    match result {
        Ok(text) => success(
            id,
            json!({ "content": [{ "type": "text", "text": text }], "isError": false }),
        ),
        Err(err) => success(
            id,
            json!({
                "content": [{ "type": "text", "text": format!("{err:#}") }],
                "isError": true
            }),
        ),
    }
}

fn resource_read_response(socket: &PathBuf, id: Value, params: &Value) -> Value {
    let uri = params
        .get("uri")
        .and_then(Value::as_str)
        .unwrap_or_default();

    let result = match uri {
        "cilux://caps" => snapshot_projection(socket, "application/json", |snapshot| {
            pretty_json(&snapshot.caps)
        }),
        "cilux://state" => snapshot_projection(socket, "application/json", |snapshot| {
            pretty_json(&snapshot.state)
        }),
        "cilux://events" => events_resource(socket, 256),
        "cilux://health" => broker_text(
            socket,
            &BrokerRequest::Health(HealthRequest::default()),
            "application/json",
        ),
        "cilux://system/dmesg" => system_resource(socket, SystemReadSelector::Dmesg),
        "cilux://system/proc_modules" => system_resource(socket, SystemReadSelector::ProcModules),
        "cilux://system/proc_meminfo" => system_resource(socket, SystemReadSelector::ProcMeminfo),
        "cilux://system/proc_loadavg" => system_resource(socket, SystemReadSelector::ProcLoadavg),
        "cilux://system/proc_uptime" => system_resource(socket, SystemReadSelector::ProcUptime),
        "cilux://system/proc_cpuinfo" => system_resource(socket, SystemReadSelector::ProcCpuinfo),
        "cilux://system/proc_interrupts" => {
            system_resource(socket, SystemReadSelector::ProcInterrupts)
        }
        "cilux://system/proc_vmstat" => system_resource(socket, SystemReadSelector::ProcVmstat),
        "cilux://system/proc_buddyinfo" => {
            system_resource(socket, SystemReadSelector::ProcBuddyinfo)
        }
        "cilux://system/proc_zoneinfo" => system_resource(socket, SystemReadSelector::ProcZoneinfo),
        _ if uri.starts_with("cilux://events/") => {
            let limit = uri
                .trim_start_matches("cilux://events/")
                .parse::<usize>()
                .context("parse event limit from resource URI");
            limit.and_then(|limit| events_resource(socket, limit))
        }
        _ if uri.starts_with("cilux://system/") => {
            let selector = uri.trim_start_matches("cilux://system/");
            parse_system_selector(selector).and_then(|selector| system_resource(socket, selector))
        }
        _ => Err(anyhow::anyhow!("unknown resource `{uri}`")),
    };

    match result {
        Ok((text, mime_type)) => success(
            id,
            json!({
                "contents": [{
                    "uri": uri,
                    "mimeType": mime_type,
                    "text": text
                }]
            }),
        ),
        Err(err) => error(id, -32000, format!("{err:#}")),
    }
}

fn broker_text(socket: &PathBuf, request: &BrokerRequest, mime_type: &'static str) -> Result<(String, &'static str)> {
    call_broker(socket, request)
        .and_then(|response| response.into_result())
        .and_then(|result| pretty_json(&result))
        .map(|text| (text, mime_type))
}

fn snapshot_projection(
    socket: &PathBuf,
    mime_type: &'static str,
    projection: impl FnOnce(cilux_common::KernelSnapshot) -> Result<String>,
) -> Result<(String, &'static str)> {
    call_broker(
        socket,
        &BrokerRequest::KernelSnapshot(KernelSnapshotRequest::default()),
    )
    .and_then(|response| response.into_result())
    .and_then(|result| match result {
        cilux_common::BrokerResult::KernelSnapshot(snapshot) => projection(snapshot),
        _ => Err(anyhow::anyhow!("unexpected broker result kind")),
    })
    .map(|text| (text, mime_type))
}

fn events_resource(socket: &PathBuf, limit: usize) -> Result<(String, &'static str)> {
    call_broker(
        socket,
        &BrokerRequest::KernelEventsTail(KernelEventsTailRequest { limit }),
    )
    .and_then(|response| response.into_result())
    .and_then(|result| match result {
        cilux_common::BrokerResult::KernelEventsTail(events) => Ok(events
            .events
            .into_iter()
            .map(|event| serde_json::to_string(&event).unwrap_or_else(|_| "{}".to_string()))
            .collect::<Vec<_>>()
            .join("\n")),
        _ => Err(anyhow::anyhow!("unexpected broker result kind")),
    })
    .map(|text| (text, "application/x-ndjson"))
}

fn system_resource(
    socket: &PathBuf,
    selector: SystemReadSelector,
) -> Result<(String, &'static str)> {
    call_broker(
        socket,
        &BrokerRequest::SystemRead(SystemReadRequest { selector }),
    )
    .and_then(|response| response.into_result())
    .and_then(|result| match result {
        cilux_common::BrokerResult::SystemRead(result) => Ok(result.text),
        _ => Err(anyhow::anyhow!("unexpected broker result kind")),
    })
    .map(|text| (text, "text/plain"))
}

fn parse_system_selector(selector: &str) -> Result<SystemReadSelector> {
    match selector {
        "dmesg" => Ok(SystemReadSelector::Dmesg),
        "proc_modules" => Ok(SystemReadSelector::ProcModules),
        "proc_meminfo" => Ok(SystemReadSelector::ProcMeminfo),
        "proc_loadavg" => Ok(SystemReadSelector::ProcLoadavg),
        "proc_uptime" => Ok(SystemReadSelector::ProcUptime),
        "proc_cpuinfo" => Ok(SystemReadSelector::ProcCpuinfo),
        "proc_interrupts" => Ok(SystemReadSelector::ProcInterrupts),
        "proc_vmstat" => Ok(SystemReadSelector::ProcVmstat),
        "proc_buddyinfo" => Ok(SystemReadSelector::ProcBuddyinfo),
        "proc_zoneinfo" => Ok(SystemReadSelector::ProcZoneinfo),
        _ => Err(anyhow::anyhow!("unknown system selector `{selector}`")),
    }
}

fn success(id: Value, result: Value) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "result": result })
}

fn error(id: Value, code: i32, message: String) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "error": { "code": code, "message": message } })
}
