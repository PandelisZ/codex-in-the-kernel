use crate::{catalog, json_rpc, tools};
use anyhow::{anyhow, Context, Result};
use cilux_common::{
    pretty_json, BrokerRequest, BrokerResult, HealthRequest, KernelEventsTailRequest,
    KernelSnapshotRequest, SystemReadRequest, SystemReadResult, SystemReadSelector,
};
use serde_json::{json, Value};
use std::path::Path;

pub(crate) fn resource_read_response(socket: &Path, id: Value, params: &Value) -> Value {
    let uri = params
        .get("uri")
        .and_then(Value::as_str)
        .unwrap_or_default();

    match read_resource(socket, uri) {
        Ok((text, mime_type)) => json_rpc::success(
            id,
            json!({
                "contents": [{
                    "uri": uri,
                    "mimeType": mime_type,
                    "text": text
                }]
            }),
        ),
        Err(err) => json_rpc::error(id, -32000, format!("{err:#}")),
    }
}

fn read_resource(socket: &Path, uri: &str) -> Result<(String, &'static str)> {
    match uri {
        "cilux://caps" => snapshot_projection(socket, |snapshot| pretty_json(&snapshot.caps)),
        "cilux://state" => snapshot_projection(socket, |snapshot| pretty_json(&snapshot.state)),
        "cilux://events" => events_resource(socket, catalog::DEFAULT_EVENT_RESOURCE_LIMIT),
        "cilux://health" => broker_json_resource(
            socket,
            &BrokerRequest::Health(HealthRequest::default()),
            "application/json",
        ),
        _ if uri.starts_with("cilux://events/") => {
            let limit = uri
                .trim_start_matches("cilux://events/")
                .parse::<usize>()
                .context("parse event limit from resource URI")?;
            events_resource(socket, limit)
        }
        _ if uri.starts_with("cilux://system/") => {
            let selector = resolve_system_resource(uri)?;
            system_resource(socket, selector)
        }
        _ => Err(anyhow!("unknown resource `{uri}`")),
    }
}

fn broker_json_resource(
    socket: &Path,
    request: &BrokerRequest,
    mime_type: &'static str,
) -> Result<(String, &'static str)> {
    tools::broker_result(socket, request)
        .and_then(|result| pretty_json(&result))
        .map(|text| (text, mime_type))
}

fn snapshot_projection(
    socket: &Path,
    projection: impl FnOnce(cilux_common::KernelSnapshot) -> Result<String>,
) -> Result<(String, &'static str)> {
    tools::broker_result(
        socket,
        &BrokerRequest::KernelSnapshot(KernelSnapshotRequest::default()),
    )
    .and_then(|result| match result {
        BrokerResult::KernelSnapshot(snapshot) => projection(snapshot),
        _ => Err(anyhow!("unexpected broker result kind")),
    })
    .map(|text| (text, "application/json"))
}

fn events_resource(socket: &Path, limit: usize) -> Result<(String, &'static str)> {
    tools::broker_result(
        socket,
        &BrokerRequest::KernelEventsTail(KernelEventsTailRequest { limit }),
    )
    .and_then(|result| match result {
        BrokerResult::KernelEventsTail(events) => Ok(events
            .events
            .into_iter()
            .map(|event| serde_json::to_string(&event).unwrap_or_else(|_| "{}".to_string()))
            .collect::<Vec<_>>()
            .join("\n")),
        _ => Err(anyhow!("unexpected broker result kind")),
    })
    .map(|text| (text, "application/x-ndjson"))
}

fn system_resource(socket: &Path, selector: SystemReadSelector) -> Result<(String, &'static str)> {
    tools::broker_result(
        socket,
        &BrokerRequest::SystemRead(SystemReadRequest { selector }),
    )
    .and_then(|result| match result {
        BrokerResult::SystemRead(SystemReadResult { text, .. }) => Ok(text),
        _ => Err(anyhow!("unexpected broker result kind")),
    })
    .map(|text| (text, "text/plain"))
}

fn resolve_system_resource(uri: &str) -> Result<SystemReadSelector> {
    if let Some(selector) = SystemReadSelector::ALL
        .into_iter()
        .find(|selector| selector.resource_uri() == uri)
    {
        return Ok(selector);
    }

    uri.trim_start_matches("cilux://system/").parse()
}

#[cfg(test)]
mod tests {
    use super::*;
    use cilux_common::BrokerResponse;
    use std::fs;
    use std::io::{BufRead, BufReader, Write};
    use std::os::unix::net::UnixListener;
    use std::thread;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn resource_uri_resolution_handles_new_explicit_and_template_uris() {
        let (socket, handle) = spawn_fake_broker(vec![
            (
                BrokerRequest::SystemRead(SystemReadRequest {
                    selector: SystemReadSelector::ProcSoftirqs,
                }),
                BrokerResponse::Ok {
                    result: BrokerResult::SystemRead(SystemReadResult {
                        selector: SystemReadSelector::ProcSoftirqs,
                        text: "softirq snapshot".to_string(),
                    }),
                },
            ),
            (
                BrokerRequest::SystemRead(SystemReadRequest {
                    selector: SystemReadSelector::ProcIomem,
                }),
                BrokerResponse::Ok {
                    result: BrokerResult::SystemRead(SystemReadResult {
                        selector: SystemReadSelector::ProcIomem,
                        text: "iomem snapshot".to_string(),
                    }),
                },
            ),
        ]);

        let explicit_response = resource_read_response(
            &socket,
            json!(1),
            &json!({ "uri": "cilux://system/proc_softirqs" }),
        );
        let templated_response = resource_read_response(
            &socket,
            json!(2),
            &json!({ "uri": "cilux://system/proc_iomem" }),
        );

        assert_eq!(
            explicit_response.pointer("/result/contents/0/text"),
            Some(&json!("softirq snapshot"))
        );
        assert_eq!(
            templated_response.pointer("/result/contents/0/text"),
            Some(&json!("iomem snapshot"))
        );
        assert_eq!(
            explicit_response.pointer("/result/contents/0/mimeType"),
            Some(&json!("text/plain"))
        );
        assert_eq!(
            templated_response.pointer("/result/contents/0/mimeType"),
            Some(&json!("text/plain"))
        );

        handle.join().expect("fake broker should exit cleanly");
        fs::remove_file(socket).ok();
    }

    fn spawn_fake_broker(
        script: Vec<(BrokerRequest, BrokerResponse)>,
    ) -> (std::path::PathBuf, thread::JoinHandle<()>) {
        let socket = std::env::temp_dir().join(format!(
            "cilux-mcp-resources-{}-{}.sock",
            std::process::id(),
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
