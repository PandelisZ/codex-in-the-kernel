use anyhow::{anyhow, bail, Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fmt;
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::path::Path;
use std::str::FromStr;

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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "method", content = "params", rename_all = "snake_case")]
pub enum BrokerRequest {
    KernelSnapshot(KernelSnapshotRequest),
    KernelEventsTail(KernelEventsTailRequest),
    TraceConfigure(TraceConfigureRequest),
    TraceStatus(TraceStatusRequest),
    TraceEnable(TraceCategoriesRequest),
    TraceDisable(TraceCategoriesRequest),
    TraceResetDefault(TraceResetDefaultRequest),
    BufferClear(BufferClearRequest),
    Health(HealthRequest),
    SystemRead(SystemReadRequest),
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct KernelSnapshotRequest {}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KernelEventsTailRequest {
    pub limit: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TraceConfigureRequest {
    pub trace_mask: u32,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TraceStatusRequest {}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TraceCategoriesRequest {
    pub categories: Vec<TraceCategory>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TraceResetDefaultRequest {}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct BufferClearRequest {}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct HealthRequest {}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SystemReadRequest {
    pub selector: SystemReadSelector,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SystemReadSelector {
    Dmesg,
    ProcCmdline,
    ProcModules,
    ProcVersion,
    ProcMeminfo,
    ProcLoadavg,
    ProcUptime,
    ProcCpuinfo,
    ProcInterrupts,
    ProcSoftirqs,
    ProcVmstat,
    ProcBuddyinfo,
    ProcZoneinfo,
    ProcIomem,
    ProcIoports,
    ProcSlabinfo,
}

impl SystemReadSelector {
    pub const ALL: [Self; 16] = [
        Self::Dmesg,
        Self::ProcCmdline,
        Self::ProcModules,
        Self::ProcVersion,
        Self::ProcMeminfo,
        Self::ProcLoadavg,
        Self::ProcUptime,
        Self::ProcCpuinfo,
        Self::ProcInterrupts,
        Self::ProcSoftirqs,
        Self::ProcVmstat,
        Self::ProcBuddyinfo,
        Self::ProcZoneinfo,
        Self::ProcIomem,
        Self::ProcIoports,
        Self::ProcSlabinfo,
    ];

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Dmesg => "dmesg",
            Self::ProcCmdline => "proc_cmdline",
            Self::ProcModules => "proc_modules",
            Self::ProcVersion => "proc_version",
            Self::ProcMeminfo => "proc_meminfo",
            Self::ProcLoadavg => "proc_loadavg",
            Self::ProcUptime => "proc_uptime",
            Self::ProcCpuinfo => "proc_cpuinfo",
            Self::ProcInterrupts => "proc_interrupts",
            Self::ProcSoftirqs => "proc_softirqs",
            Self::ProcVmstat => "proc_vmstat",
            Self::ProcBuddyinfo => "proc_buddyinfo",
            Self::ProcZoneinfo => "proc_zoneinfo",
            Self::ProcIomem => "proc_iomem",
            Self::ProcIoports => "proc_ioports",
            Self::ProcSlabinfo => "proc_slabinfo",
        }
    }

    pub const fn resource_uri(self) -> &'static str {
        match self {
            Self::Dmesg => "cilux://system/dmesg",
            Self::ProcCmdline => "cilux://system/proc_cmdline",
            Self::ProcModules => "cilux://system/proc_modules",
            Self::ProcVersion => "cilux://system/proc_version",
            Self::ProcMeminfo => "cilux://system/proc_meminfo",
            Self::ProcLoadavg => "cilux://system/proc_loadavg",
            Self::ProcUptime => "cilux://system/proc_uptime",
            Self::ProcCpuinfo => "cilux://system/proc_cpuinfo",
            Self::ProcInterrupts => "cilux://system/proc_interrupts",
            Self::ProcSoftirqs => "cilux://system/proc_softirqs",
            Self::ProcVmstat => "cilux://system/proc_vmstat",
            Self::ProcBuddyinfo => "cilux://system/proc_buddyinfo",
            Self::ProcZoneinfo => "cilux://system/proc_zoneinfo",
            Self::ProcIomem => "cilux://system/proc_iomem",
            Self::ProcIoports => "cilux://system/proc_ioports",
            Self::ProcSlabinfo => "cilux://system/proc_slabinfo",
        }
    }

    pub const fn resource_name(self) -> &'static str {
        match self {
            Self::Dmesg => "Guest Dmesg",
            Self::ProcCmdline => "Guest Proc Cmdline",
            Self::ProcModules => "Guest Proc Modules",
            Self::ProcVersion => "Guest Proc Version",
            Self::ProcMeminfo => "Guest Proc Meminfo",
            Self::ProcLoadavg => "Guest Proc Loadavg",
            Self::ProcUptime => "Guest Proc Uptime",
            Self::ProcCpuinfo => "Guest Proc Cpuinfo",
            Self::ProcInterrupts => "Guest Proc Interrupts",
            Self::ProcSoftirqs => "Guest Proc Softirqs",
            Self::ProcVmstat => "Guest Proc Vmstat",
            Self::ProcBuddyinfo => "Guest Proc Buddyinfo",
            Self::ProcZoneinfo => "Guest Proc Zoneinfo",
            Self::ProcIomem => "Guest Proc Iomem",
            Self::ProcIoports => "Guest Proc Ioports",
            Self::ProcSlabinfo => "Guest Proc Slabinfo",
        }
    }

    pub const fn resource_description(self) -> &'static str {
        match self {
            Self::Dmesg => "Kernel ring buffer via the broker's curated system read path.",
            Self::ProcCmdline => "Current boot arguments from /proc/cmdline.",
            Self::ProcModules => "Current loaded kernel modules from /proc/modules.",
            Self::ProcVersion => "Kernel build identity from /proc/version.",
            Self::ProcMeminfo => "Current memory accounting from /proc/meminfo.",
            Self::ProcLoadavg => "Current scheduler load from /proc/loadavg.",
            Self::ProcUptime => "Current uptime from /proc/uptime.",
            Self::ProcCpuinfo => "Current CPU topology and features from /proc/cpuinfo.",
            Self::ProcInterrupts => "Current interrupt counters from /proc/interrupts.",
            Self::ProcSoftirqs => "Current softirq counters from /proc/softirqs.",
            Self::ProcVmstat => "Current virtual-memory counters from /proc/vmstat.",
            Self::ProcBuddyinfo => "Current page allocator free-area state from /proc/buddyinfo.",
            Self::ProcZoneinfo => {
                "Current zone watermarks and page accounting from /proc/zoneinfo."
            }
            Self::ProcIomem => "Current physical memory map from /proc/iomem.",
            Self::ProcIoports => "Current I/O port allocations from /proc/ioports.",
            Self::ProcSlabinfo => "Current slab allocator state from /proc/slabinfo.",
        }
    }
}

impl FromStr for SystemReadSelector {
    type Err = anyhow::Error;

    fn from_str(selector: &str) -> Result<Self, Self::Err> {
        match selector {
            "dmesg" => Ok(Self::Dmesg),
            "proc_cmdline" => Ok(Self::ProcCmdline),
            "proc_modules" => Ok(Self::ProcModules),
            "proc_version" => Ok(Self::ProcVersion),
            "proc_meminfo" => Ok(Self::ProcMeminfo),
            "proc_loadavg" => Ok(Self::ProcLoadavg),
            "proc_uptime" => Ok(Self::ProcUptime),
            "proc_cpuinfo" => Ok(Self::ProcCpuinfo),
            "proc_interrupts" => Ok(Self::ProcInterrupts),
            "proc_softirqs" => Ok(Self::ProcSoftirqs),
            "proc_vmstat" => Ok(Self::ProcVmstat),
            "proc_buddyinfo" => Ok(Self::ProcBuddyinfo),
            "proc_zoneinfo" => Ok(Self::ProcZoneinfo),
            "proc_iomem" => Ok(Self::ProcIomem),
            "proc_ioports" => Ok(Self::ProcIoports),
            "proc_slabinfo" => Ok(Self::ProcSlabinfo),
            _ => Err(anyhow!("unknown system selector `{selector}`")),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TraceCategory {
    SchedProcessExec,
    SchedProcessExit,
    Module,
    MarkVictim,
}

impl TraceCategory {
    pub const ALL: [Self; 4] = [
        Self::SchedProcessExec,
        Self::SchedProcessExit,
        Self::Module,
        Self::MarkVictim,
    ];

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::SchedProcessExec => "sched_process_exec",
            Self::SchedProcessExit => "sched_process_exit",
            Self::Module => "module",
            Self::MarkVictim => "mark_victim",
        }
    }

    pub const fn mask_bit(self) -> u32 {
        match self {
            Self::SchedProcessExec => TRACE_EXEC,
            Self::SchedProcessExit => TRACE_EXIT,
            Self::Module => TRACE_MODULE,
            Self::MarkVictim => TRACE_OOM,
        }
    }

    pub fn from_mask(mask: u32) -> Vec<Self> {
        Self::ALL
            .into_iter()
            .filter(|category| mask & category.mask_bit() != 0)
            .collect()
    }

    pub fn mask_for(categories: impl IntoIterator<Item = Self>) -> u32 {
        categories
            .into_iter()
            .fold(0, |mask, category| mask | category.mask_bit())
    }
}

impl FromStr for TraceCategory {
    type Err = anyhow::Error;

    fn from_str(category: &str) -> Result<Self, Self::Err> {
        match category {
            "sched_process_exec" => Ok(Self::SchedProcessExec),
            "sched_process_exit" => Ok(Self::SchedProcessExit),
            "module" => Ok(Self::Module),
            "mark_victim" => Ok(Self::MarkVictim),
            _ => Err(anyhow!("unknown trace category `{category}`")),
        }
    }
}

impl fmt::Display for SystemReadSelector {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl fmt::Display for TraceCategory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
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
    TraceStatus(TraceStatusResult),
    TraceEnable(TraceStatusResult),
    TraceDisable(TraceStatusResult),
    TraceResetDefault(TraceStatusResult),
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TraceStatusResult {
    pub trace_mask: u32,
    pub supported_mask: u32,
    pub drop_count: u32,
    pub event_count: u32,
    pub ring_capacity: u32,
    pub enabled_categories: Vec<TraceCategory>,
    pub supported_categories: Vec<TraceCategory>,
}

impl TraceStatusResult {
    pub fn new(
        trace_mask: u32,
        supported_mask: u32,
        drop_count: u32,
        event_count: u32,
        ring_capacity: u32,
    ) -> Self {
        Self {
            trace_mask,
            supported_mask,
            drop_count,
            event_count,
            ring_capacity,
            enabled_categories: TraceCategory::from_mask(trace_mask),
            supported_categories: TraceCategory::from_mask(supported_mask),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StatusResult {
    pub ok: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HealthReport {
    pub broker_pid: u32,
    pub socket_path: String,
    pub audit_log_path: String,
    pub debugfs_ready: bool,
    pub netlink_ready: bool,
    pub app_server_port: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn system_read_selectors_round_trip() {
        let selectors = SystemReadSelector::ALL
            .into_iter()
            .map(|selector| {
                let parsed = selector
                    .as_str()
                    .parse::<SystemReadSelector>()
                    .expect("selector should parse");
                (selector, parsed)
            })
            .collect::<Vec<_>>();

        assert_eq!(
            selectors,
            SystemReadSelector::ALL
                .into_iter()
                .map(|selector| (selector, selector))
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn trace_categories_round_trip_and_mask_conversion() {
        let categories = TraceCategory::ALL
            .into_iter()
            .map(|category| {
                let parsed = category
                    .as_str()
                    .parse::<TraceCategory>()
                    .expect("category should parse");
                (category, parsed)
            })
            .collect::<Vec<_>>();

        assert_eq!(
            categories,
            TraceCategory::ALL
                .into_iter()
                .map(|category| (category, category))
                .collect::<Vec<_>>()
        );

        let mask = TraceCategory::mask_for([
            TraceCategory::SchedProcessExec,
            TraceCategory::Module,
            TraceCategory::SchedProcessExec,
        ]);
        assert_eq!(mask, TRACE_EXEC | TRACE_MODULE);
        assert_eq!(
            TraceCategory::from_mask(mask),
            vec![TraceCategory::SchedProcessExec, TraceCategory::Module]
        );
    }

    #[test]
    fn trace_status_result_derives_enabled_and_supported_categories() {
        assert_eq!(
            TraceStatusResult::new(TRACE_EXEC | TRACE_MODULE, TRACE_DEFAULT_MASK, 3, 5, 16),
            TraceStatusResult {
                trace_mask: TRACE_EXEC | TRACE_MODULE,
                supported_mask: TRACE_DEFAULT_MASK,
                drop_count: 3,
                event_count: 5,
                ring_capacity: 16,
                enabled_categories: vec![TraceCategory::SchedProcessExec, TraceCategory::Module,],
                supported_categories: vec![
                    TraceCategory::SchedProcessExec,
                    TraceCategory::SchedProcessExit,
                    TraceCategory::Module,
                    TraceCategory::MarkVictim,
                ],
            }
        );
    }
}
