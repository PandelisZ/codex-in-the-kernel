mod audit;
mod kernel;
mod netlink;

use anyhow::{Context, Result};
use audit::AuditLog;
use cilux_common::{
    BrokerError, BrokerRequest, BrokerResponse, BrokerResult, DEFAULT_AUDIT_LOG,
    DEFAULT_BROKER_SOCKET, DEFAULT_DEBUGFS_ROOT,
};
use clap::Parser;
use kernel::KernelFacade;
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::os::unix::fs::PermissionsExt;
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::PathBuf;
use std::sync::Arc;
use std::thread;

#[derive(Parser, Debug)]
struct Args {
    #[arg(long, default_value = DEFAULT_BROKER_SOCKET)]
    socket: PathBuf,
    #[arg(long, default_value = DEFAULT_AUDIT_LOG)]
    audit_log: PathBuf,
    #[arg(long, default_value = DEFAULT_DEBUGFS_ROOT)]
    debugfs_root: PathBuf,
}

struct ServerState {
    kernel: KernelFacade,
    audit: AuditLog,
    socket_path: PathBuf,
}

fn main() -> Result<()> {
    let args = Args::parse();
    let audit = AuditLog::open(&args.audit_log)?;
    let kernel = KernelFacade::new(&args.debugfs_root, audit.path());

    if args.socket.exists() {
        fs::remove_file(&args.socket)
            .with_context(|| format!("remove stale socket {}", args.socket.display()))?;
    }
    if let Some(parent) = args.socket.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("create socket parent {}", parent.display()))?;
    }

    let listener = UnixListener::bind(&args.socket)
        .with_context(|| format!("bind {}", args.socket.display()))?;
    fs::set_permissions(&args.socket, fs::Permissions::from_mode(0o660))
        .with_context(|| format!("chmod {}", args.socket.display()))?;

    let state = Arc::new(ServerState {
        kernel,
        audit,
        socket_path: args.socket,
    });

    for incoming in listener.incoming() {
        let stream = incoming.context("accept broker connection")?;
        let state = Arc::clone(&state);
        thread::spawn(move || {
            if let Err(err) = handle_client(stream, &state) {
                let _ =
                    state
                        .audit
                        .write_entry("connection", false, format!("handler error: {err:#}"));
            }
        });
    }

    Ok(())
}

fn handle_client(mut stream: UnixStream, state: &ServerState) -> Result<()> {
    let mut line = String::new();
    {
        let mut reader = BufReader::new(
            stream
                .try_clone()
                .context("clone unix stream for buffered reading")?,
        );
        let read = reader.read_line(&mut line).context("read broker request")?;
        if read == 0 {
            return Ok(());
        }
    }

    let response = match serde_json::from_str::<BrokerRequest>(&line) {
        Ok(request) => dispatch_request(state, request),
        Err(err) => {
            let error = BrokerError {
                code: "bad_request".to_string(),
                message: format!("invalid request JSON: {err}"),
            };
            BrokerResponse::Error { error }
        }
    };

    serde_json::to_writer(&mut stream, &response).context("serialize broker response")?;
    stream
        .write_all(b"\n")
        .context("write broker response newline")?;
    Ok(())
}

fn dispatch_request(state: &ServerState, request: BrokerRequest) -> BrokerResponse {
    let method = request_method(&request);
    match dispatch_request_inner(state, request) {
        Ok(result) => {
            let _ = state.audit.write_entry(method, true, "ok");
            BrokerResponse::Ok { result }
        }
        Err(err) => {
            let _ = state.audit.write_entry(method, false, format!("{err:#}"));
            BrokerResponse::Error {
                error: BrokerError {
                    code: "request_failed".to_string(),
                    message: err.to_string(),
                },
            }
        }
    }
}

fn dispatch_request_inner(state: &ServerState, request: BrokerRequest) -> Result<BrokerResult> {
    Ok(match request {
        BrokerRequest::KernelSnapshot(_) => {
            BrokerResult::KernelSnapshot(state.kernel.kernel_snapshot()?)
        }
        BrokerRequest::KernelEventsTail(params) => {
            BrokerResult::KernelEventsTail(state.kernel.kernel_events_tail(params.limit)?)
        }
        BrokerRequest::TraceConfigure(params) => {
            BrokerResult::TraceConfigure(state.kernel.trace_configure(params.trace_mask)?)
        }
        BrokerRequest::TraceStatus(_) => BrokerResult::TraceStatus(state.kernel.trace_status()?),
        BrokerRequest::TraceEnable(params) => {
            BrokerResult::TraceEnable(state.kernel.trace_enable(&params.categories)?)
        }
        BrokerRequest::TraceDisable(params) => {
            BrokerResult::TraceDisable(state.kernel.trace_disable(&params.categories)?)
        }
        BrokerRequest::TraceResetDefault(_) => {
            BrokerResult::TraceResetDefault(state.kernel.trace_reset_default()?)
        }
        BrokerRequest::BufferClear(_) => BrokerResult::BufferClear(state.kernel.buffer_clear()?),
        BrokerRequest::Health(_) => {
            BrokerResult::Health(state.kernel.health(std::process::id(), &state.socket_path))
        }
        BrokerRequest::SystemRead(params) => {
            BrokerResult::SystemRead(state.kernel.system_read(params.selector)?)
        }
    })
}

fn request_method(request: &BrokerRequest) -> &'static str {
    match request {
        BrokerRequest::KernelSnapshot(_) => "kernel_snapshot",
        BrokerRequest::KernelEventsTail(_) => "kernel_events_tail",
        BrokerRequest::TraceConfigure(_) => "trace_configure",
        BrokerRequest::TraceStatus(_) => "trace_status",
        BrokerRequest::TraceEnable(_) => "trace_enable",
        BrokerRequest::TraceDisable(_) => "trace_disable",
        BrokerRequest::TraceResetDefault(_) => "trace_reset_default",
        BrokerRequest::BufferClear(_) => "buffer_clear",
        BrokerRequest::Health(_) => "health",
        BrokerRequest::SystemRead(_) => "system_read",
    }
}
