use crate::{catalog, json_rpc};
use anyhow::{anyhow, bail, Result};
use cilux_common::{
    call_broker, pretty_json, BrokerRequest, BrokerResult, BufferClearRequest, HealthRequest,
    KernelEventsTailRequest, KernelSnapshotRequest, SystemReadRequest, SystemReadSelector,
    TraceCategoriesRequest, TraceCategory, TraceConfigureRequest, TraceResetDefaultRequest,
    TraceStatusRequest,
};
use serde_json::{json, Value};
use std::path::Path;

pub(crate) fn tool_call_response(socket: &Path, id: Value, params: &Value) -> Value {
    let name = params
        .get("name")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let arguments = params
        .get("arguments")
        .cloned()
        .unwrap_or_else(|| json!({}));
    let capabilities = catalog::CatalogCapabilities::detect(socket);

    let known_tool = matches!(
        name,
        "cilux_kernel_snapshot"
            | "cilux_events_tail"
            | "cilux_trace_configure"
            | "cilux_trace_status"
            | "cilux_trace_enable"
            | "cilux_trace_disable"
            | "cilux_trace_reset_default"
            | "cilux_health"
            | "cilux_buffer_clear"
            | "cilux_system_read"
    );

    let result = if name.is_empty() {
        Err(anyhow!("missing tool name"))
    } else if !known_tool {
        Err(anyhow!("unknown tool `{name}`"))
    } else if !capabilities.supports_tool(name) {
        Err(anyhow!(
            "tool `{name}` is unavailable in the current guest mode"
        ))
    } else {
        match name {
            "cilux_kernel_snapshot" => broker_pretty_json(
                socket,
                &BrokerRequest::KernelSnapshot(KernelSnapshotRequest::default()),
            ),
            "cilux_events_tail" => {
                let limit = arguments
                    .get("limit")
                    .and_then(Value::as_u64)
                    .unwrap_or(catalog::DEFAULT_EVENTS_LIMIT as u64)
                    as usize;
                broker_pretty_json(
                    socket,
                    &BrokerRequest::KernelEventsTail(KernelEventsTailRequest { limit }),
                )
            }
            "cilux_trace_configure" => {
                let trace_mask = arguments
                    .get("trace_mask")
                    .and_then(Value::as_u64)
                    .unwrap_or_default() as u32;
                broker_pretty_json(
                    socket,
                    &BrokerRequest::TraceConfigure(TraceConfigureRequest { trace_mask }),
                )
            }
            "cilux_trace_status" => broker_pretty_json(
                socket,
                &BrokerRequest::TraceStatus(TraceStatusRequest::default()),
            ),
            "cilux_trace_enable" => parse_trace_categories(&arguments).and_then(|categories| {
                broker_pretty_json(
                    socket,
                    &BrokerRequest::TraceEnable(TraceCategoriesRequest { categories }),
                )
            }),
            "cilux_trace_disable" => parse_trace_categories(&arguments).and_then(|categories| {
                broker_pretty_json(
                    socket,
                    &BrokerRequest::TraceDisable(TraceCategoriesRequest { categories }),
                )
            }),
            "cilux_trace_reset_default" => broker_pretty_json(
                socket,
                &BrokerRequest::TraceResetDefault(TraceResetDefaultRequest::default()),
            ),
            "cilux_health" => {
                broker_pretty_json(socket, &BrokerRequest::Health(HealthRequest::default()))
            }
            "cilux_buffer_clear" => broker_pretty_json(
                socket,
                &BrokerRequest::BufferClear(BufferClearRequest::default()),
            ),
            "cilux_system_read" => parse_system_selector(&arguments).and_then(|selector| {
                broker_pretty_json(
                    socket,
                    &BrokerRequest::SystemRead(SystemReadRequest { selector }),
                )
            }),
            _ => unreachable!("known tool dispatch should be exhaustive"),
        }
    };

    match result {
        Ok(text) => json_rpc::success(
            id,
            json!({ "content": [{ "type": "text", "text": text }], "isError": false }),
        ),
        Err(err) => json_rpc::success(
            id,
            json!({
                "content": [{ "type": "text", "text": format!("{err:#}") }],
                "isError": true
            }),
        ),
    }
}

pub(crate) fn broker_result(socket: &Path, request: &BrokerRequest) -> Result<BrokerResult> {
    call_broker(socket, request).and_then(|response| response.into_result())
}

fn broker_pretty_json(socket: &Path, request: &BrokerRequest) -> Result<String> {
    broker_result(socket, request).and_then(|result| pretty_json(&result))
}

fn parse_system_selector(arguments: &Value) -> Result<SystemReadSelector> {
    arguments
        .get("selector")
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow!("missing selector"))?
        .parse()
}

fn parse_trace_categories(arguments: &Value) -> Result<Vec<TraceCategory>> {
    let categories = arguments
        .get("categories")
        .and_then(Value::as_array)
        .ok_or_else(|| anyhow!("missing categories"))?;
    if categories.is_empty() {
        bail!("categories must not be empty");
    }

    categories
        .iter()
        .map(|category| {
            category
                .as_str()
                .ok_or_else(|| anyhow!("categories must be strings"))?
                .parse::<TraceCategory>()
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use cilux_common::{BrokerError, BrokerResponse, TraceStatusResult, TRACE_EXEC, TRACE_MODULE};
    use std::fs;
    use std::io::{BufRead, BufReader, Write};
    use std::os::unix::net::UnixListener;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::thread;
    use std::time::{SystemTime, UNIX_EPOCH};

    static NEXT_SOCKET_ID: AtomicU64 = AtomicU64::new(0);

    #[test]
    fn tool_handlers_wrap_broker_success_and_error() {
        let (socket, handle) = spawn_fake_broker(vec![
            (
                BrokerRequest::Health(HealthRequest::default()),
                BrokerResponse::Ok {
                    result: BrokerResult::Health(cilux_common::HealthReport {
                        broker_pid: 7,
                        socket_path: "/run/cilux-broker.sock".to_string(),
                        audit_log_path: "/var/log/cilux-broker.log".to_string(),
                        guest_mode: cilux_common::GuestMode::ResearchKernel,
                        debugfs_ready: true,
                        netlink_ready: true,
                        app_server_port: 8765,
                        capabilities: cilux_common::BrokerCapabilities::full(),
                    }),
                },
            ),
            (
                BrokerRequest::TraceStatus(TraceStatusRequest::default()),
                BrokerResponse::Ok {
                    result: BrokerResult::TraceStatus(TraceStatusResult::new(
                        TRACE_EXEC | TRACE_MODULE,
                        TRACE_EXEC | TRACE_MODULE,
                        1,
                        2,
                        16,
                    )),
                },
            ),
            (
                BrokerRequest::Health(HealthRequest::default()),
                BrokerResponse::Ok {
                    result: BrokerResult::Health(cilux_common::HealthReport {
                        broker_pid: 7,
                        socket_path: "/run/cilux-broker.sock".to_string(),
                        audit_log_path: "/var/log/cilux-broker.log".to_string(),
                        guest_mode: cilux_common::GuestMode::ResearchKernel,
                        debugfs_ready: true,
                        netlink_ready: true,
                        app_server_port: 8765,
                        capabilities: cilux_common::BrokerCapabilities::full(),
                    }),
                },
            ),
            (
                BrokerRequest::TraceEnable(TraceCategoriesRequest {
                    categories: vec![TraceCategory::Module],
                }),
                BrokerResponse::Error {
                    error: BrokerError {
                        code: "request_failed".to_string(),
                        message: "unsupported trace categories requested: module".to_string(),
                    },
                },
            ),
        ]);

        let success_response = tool_call_response(
            &socket,
            json!(1),
            &json!({ "name": "cilux_trace_status", "arguments": {} }),
        );
        let error_response = tool_call_response(
            &socket,
            json!(2),
            &json!({
                "name": "cilux_trace_enable",
                "arguments": { "categories": ["module"] }
            }),
        );

        assert_eq!(success_response["result"]["isError"], json!(false));
        assert_eq!(error_response["result"]["isError"], json!(true));
        assert_eq!(
            success_response.pointer("/result/content/0/type"),
            Some(&json!("text"))
        );
        assert!(success_response
            .pointer("/result/content/0/text")
            .and_then(Value::as_str)
            .expect("success response should contain text")
            .contains("\"enabled_categories\""));
        assert_eq!(
            error_response.pointer("/result/content/0/text"),
            Some(&json!(
                "request_failed: unsupported trace categories requested: module"
            ))
        );

        handle.join().expect("fake broker should exit cleanly");
        fs::remove_file(socket).ok();
    }

    #[test]
    fn desktop_mode_rejects_unavailable_trace_tools() {
        let (socket, handle) = spawn_fake_broker(vec![(
            BrokerRequest::Health(HealthRequest::default()),
            BrokerResponse::Ok {
                result: BrokerResult::Health(cilux_common::HealthReport {
                    broker_pid: 11,
                    socket_path: "/run/cilux-broker.sock".to_string(),
                    audit_log_path: "/var/log/cilux-broker.log".to_string(),
                    guest_mode: cilux_common::GuestMode::DesktopStockKernel,
                    debugfs_ready: false,
                    netlink_ready: false,
                    app_server_port: 8765,
                    capabilities: cilux_common::BrokerCapabilities::desktop_stock_kernel(),
                }),
            },
        )]);

        let response = tool_call_response(
            &socket,
            json!(3),
            &json!({ "name": "cilux_trace_status", "arguments": {} }),
        );

        assert_eq!(response["result"]["isError"], json!(true));
        assert_eq!(
            response.pointer("/result/content/0/text"),
            Some(&json!(
                "tool `cilux_trace_status` is unavailable in the current guest mode"
            ))
        );

        handle.join().expect("fake broker should exit cleanly");
        fs::remove_file(socket).ok();
    }

    fn spawn_fake_broker(
        script: Vec<(BrokerRequest, BrokerResponse)>,
    ) -> (std::path::PathBuf, thread::JoinHandle<()>) {
        let socket = std::env::temp_dir().join(format!(
            "cilux-mcp-tools-{}-{}-{}.sock",
            std::process::id(),
            NEXT_SOCKET_ID.fetch_add(1, Ordering::Relaxed),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("time should be monotonic enough")
                .as_nanos()
        ));
        let listener = UnixListener::bind(&socket).expect("socket should bind");
        let handle = thread::spawn(move || {
            for (expected_request, response) in script {
                let (mut stream, _) = listener.accept().expect("broker should accept connection");
                let mut line = String::new();
                BufReader::new(stream.try_clone().expect("stream clone should succeed"))
                    .read_line(&mut line)
                    .expect("broker should read request");
                let request: BrokerRequest =
                    serde_json::from_str(&line).expect("request should deserialize");
                assert_eq!(request, expected_request);
                serde_json::to_writer(&mut stream, &response).expect("response should serialize");
                stream
                    .write_all(b"\n")
                    .expect("broker should write response newline");
            }
        });
        (socket, handle)
    }
}
