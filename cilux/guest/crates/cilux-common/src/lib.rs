use anyhow::{anyhow, bail, Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fmt;
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::path::Path;

pub const DEFAULT_BROKER_SOCKET: &str = "/run/cilux-broker.sock";
pub const DEFAULT_DEBUGFS_ROOT: &str = "/sys/kernel/debug/cilux";
pub const DEFAULT_AUDIT_LOG: &str = "/var/log/cilux-broker.log";
pub const DEFAULT_APP_SERVER_PORT: u16 = 8765;

pub const TRACE_EXEC: u32 = 1 << 0;
pub const TRACE_EXIT: u32 = 1 << 1;
pub const TRACE_MODULE: u32 = 1 << 2;
pub const TRACE_OOM: u32 = 1 << 3;
pub const TRACE_DEFAULT_MASK: u32 = TRACE_EXEC | TRACE_EXIT | TRACE_MODULE | TRACE_OOM;

pub const FAMILY_NAME: &str = "cilux";
pub const FAMILY_VERSION: u8 = 1;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "method", content = "params", rename_all = "snake_case")]
pub enum BrokerRequest {
    KernelSnapshot(KernelSnapshotRequest),
    KernelEventsTail(KernelEventsTailRequest),
    TraceConfigure(TraceConfigureRequest),
    BufferClear(BufferClearRequest),
    Health(HealthRequest),
    SystemRead(SystemReadRequest),
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct KernelSnapshotRequest {}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KernelEventsTailRequest {
    pub limit: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceConfigureRequest {
    pub trace_mask: u32,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BufferClearRequest {}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct HealthRequest {}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemReadRequest {
    pub selector: SystemReadSelector,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SystemReadSelector {
    Dmesg,
    ProcModules,
    ProcMeminfo,
    ProcLoadavg,
    ProcUptime,
    ProcCpuinfo,
    ProcInterrupts,
    ProcVmstat,
    ProcBuddyinfo,
    ProcZoneinfo,
}

impl fmt::Display for SystemReadSelector {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let text = match self {
            Self::Dmesg => "dmesg",
            Self::ProcModules => "proc_modules",
            Self::ProcMeminfo => "proc_meminfo",
            Self::ProcLoadavg => "proc_loadavg",
            Self::ProcUptime => "proc_uptime",
            Self::ProcCpuinfo => "proc_cpuinfo",
            Self::ProcInterrupts => "proc_interrupts",
            Self::ProcVmstat => "proc_vmstat",
            Self::ProcBuddyinfo => "proc_buddyinfo",
            Self::ProcZoneinfo => "proc_zoneinfo",
        };
        f.write_str(text)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum BrokerResponse {
    Ok { result: BrokerResult },
    Error { error: BrokerError },
}

impl BrokerResponse {
    pub fn into_result(self) -> Result<BrokerResult> {
        match self {
            Self::Ok { result } => Ok(result),
            Self::Error { error } => bail!("{}: {}", error.code, error.message),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrokerError {
    pub code: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", content = "data", rename_all = "snake_case")]
pub enum BrokerResult {
    KernelSnapshot(KernelSnapshot),
    KernelEventsTail(KernelEventsTail),
    TraceConfigure(TraceConfigureResult),
    BufferClear(StatusResult),
    Health(HealthReport),
    SystemRead(SystemReadResult),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KernelSnapshot {
    pub caps: Value,
    pub state: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KernelEventsTail {
    pub events: Vec<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceConfigureResult {
    pub trace_mask: u32,
    pub state: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatusResult {
    pub ok: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthReport {
    pub broker_pid: u32,
    pub socket_path: String,
    pub audit_log_path: String,
    pub debugfs_ready: bool,
    pub netlink_ready: bool,
    pub app_server_port: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemReadResult {
    pub selector: SystemReadSelector,
    pub text: String,
}

pub fn call_broker(
    socket_path: impl AsRef<Path>,
    request: &BrokerRequest,
) -> Result<BrokerResponse> {
    let socket_path = socket_path.as_ref();
    let mut stream = UnixStream::connect(socket_path)
        .with_context(|| format!("failed to connect to broker at {}", socket_path.display()))?;
    let payload = serde_json::to_vec(request).context("serialize broker request")?;
    stream
        .write_all(&payload)
        .context("write broker request payload")?;
    stream
        .write_all(b"\n")
        .context("write broker request newline")?;
    stream.flush().context("flush broker request")?;

    let mut reader = BufReader::new(stream);
    let mut line = String::new();
    let read = reader
        .read_line(&mut line)
        .context("read broker response line")?;
    if read == 0 {
        bail!("broker closed the connection without sending a response");
    }

    serde_json::from_str(&line).context("deserialize broker response")
}

pub fn expect_kind<T>(
    response: BrokerResponse,
    f: impl FnOnce(BrokerResult) -> Option<T>,
) -> Result<T> {
    let result = response.into_result()?;
    f(result).ok_or_else(|| anyhow!("broker returned an unexpected result kind"))
}

pub fn pretty_json(value: &impl Serialize) -> Result<String> {
    serde_json::to_string_pretty(value).context("serialize pretty JSON")
}
