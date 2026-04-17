use crate::netlink;
use anyhow::{bail, Context, Result};
use cilux_common::{
    HealthReport, KernelEventsTail, KernelSnapshot, StatusResult, SystemReadResult,
    SystemReadSelector, TraceCategory, TraceConfigureResult, TraceStatusResult,
    DEFAULT_APP_SERVER_PORT, DEFAULT_DEBUGFS_ROOT, TRACE_DEFAULT_MASK,
};
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

pub struct KernelFacade {
    debugfs_root: PathBuf,
    audit_log_path: PathBuf,
}

impl KernelFacade {
    pub fn new(debugfs_root: impl AsRef<Path>, audit_log_path: impl AsRef<Path>) -> Self {
        Self {
            debugfs_root: debugfs_root.as_ref().to_path_buf(),
            audit_log_path: audit_log_path.as_ref().to_path_buf(),
        }
    }

    pub fn caps_path(&self) -> PathBuf {
        self.debugfs_root.join("caps.json")
    }

    pub fn state_path(&self) -> PathBuf {
        self.debugfs_root.join("state.json")
    }

    pub fn events_path(&self) -> PathBuf {
        self.debugfs_root.join("events.ndjson")
    }

    pub fn kernel_snapshot(&self) -> Result<KernelSnapshot> {
        Ok(KernelSnapshot {
            caps: read_json_file(self.caps_path())?,
            state: read_json_file(self.state_path())?,
        })
    }

    pub fn kernel_events_tail(&self, limit: usize) -> Result<KernelEventsTail> {
        let data = fs::read_to_string(self.events_path()).context("read events.ndjson")?;
        let mut events = Vec::new();
        for line in data
            .lines()
            .rev()
            .take(limit)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
        {
            events.push(
                serde_json::from_str::<Value>(line)
                    .with_context(|| format!("parse event line `{line}`"))?,
            );
        }
        Ok(KernelEventsTail { events })
    }

    pub fn trace_configure(&self, trace_mask: u32) -> Result<TraceConfigureResult> {
        let trace_mask = netlink::set_trace_mask(trace_mask)?;
        Ok(TraceConfigureResult {
            trace_mask,
            state: read_json_file(self.state_path())?,
        })
    }

    pub fn trace_status(&self) -> Result<TraceStatusResult> {
        netlink::get_state().map(trace_status_from_kernel_state)
    }

    pub fn trace_enable(&self, categories: &[TraceCategory]) -> Result<TraceStatusResult> {
        let state = netlink::get_state()?;
        let trace_mask = trace_enable_mask(state.trace_mask, state.supported_mask, categories)?;
        self.trace_update(trace_mask)
    }

    pub fn trace_disable(&self, categories: &[TraceCategory]) -> Result<TraceStatusResult> {
        let state = netlink::get_state()?;
        let trace_mask = trace_disable_mask(state.trace_mask, state.supported_mask, categories)?;
        self.trace_update(trace_mask)
    }

    pub fn trace_reset_default(&self) -> Result<TraceStatusResult> {
        let state = netlink::get_state()?;
        self.trace_update(trace_reset_default_mask(state.supported_mask))
    }

    pub fn buffer_clear(&self) -> Result<StatusResult> {
        let remaining = netlink::clear_events()?;
        Ok(StatusResult { ok: remaining == 0 })
    }

    pub fn health(&self, broker_pid: u32, socket_path: &Path) -> HealthReport {
        let debugfs_ready =
            self.caps_path().exists() && self.state_path().exists() && self.events_path().exists();
        let netlink_ready = netlink::ping().is_ok();

        HealthReport {
            broker_pid,
            socket_path: socket_path.display().to_string(),
            audit_log_path: self.audit_log_path.display().to_string(),
            debugfs_ready,
            netlink_ready,
            app_server_port: DEFAULT_APP_SERVER_PORT,
        }
    }

    pub fn system_read(&self, selector: SystemReadSelector) -> Result<SystemReadResult> {
        let text = match system_read_path(selector) {
            Some(path) => read_text_file(path)?,
            None => run_read_command("dmesg", &[])?,
        };

        Ok(SystemReadResult { selector, text })
    }

    fn trace_update(&self, trace_mask: u32) -> Result<TraceStatusResult> {
        netlink::set_trace_mask(trace_mask)?;
        self.trace_status()
    }
}

fn trace_status_from_kernel_state(state: netlink::KernelState) -> TraceStatusResult {
    TraceStatusResult::new(
        state.trace_mask,
        state.supported_mask,
        state.drop_count,
        state.event_count,
        state.ring_capacity,
    )
}

fn trace_enable_mask(
    current_mask: u32,
    supported_mask: u32,
    categories: &[TraceCategory],
) -> Result<u32> {
    let requested_mask = trace_categories_mask(categories)?;
    ensure_supported_categories(supported_mask, requested_mask)?;
    Ok(current_mask | requested_mask)
}

fn trace_disable_mask(
    current_mask: u32,
    supported_mask: u32,
    categories: &[TraceCategory],
) -> Result<u32> {
    let requested_mask = trace_categories_mask(categories)?;
    ensure_supported_categories(supported_mask, requested_mask)?;
    Ok(current_mask & !requested_mask)
}

fn trace_reset_default_mask(supported_mask: u32) -> u32 {
    TRACE_DEFAULT_MASK & supported_mask
}

fn trace_categories_mask(categories: &[TraceCategory]) -> Result<u32> {
    if categories.is_empty() {
        bail!("trace categories must not be empty");
    }
    Ok(TraceCategory::mask_for(categories.iter().copied()))
}

fn ensure_supported_categories(supported_mask: u32, requested_mask: u32) -> Result<()> {
    let unsupported_mask = requested_mask & !supported_mask;
    if unsupported_mask == 0 {
        return Ok(());
    }

    let unsupported_categories = TraceCategory::from_mask(unsupported_mask)
        .into_iter()
        .map(|category| category.to_string())
        .collect::<Vec<_>>()
        .join(", ");
    bail!("unsupported trace categories requested: {unsupported_categories}");
}

fn system_read_path(selector: SystemReadSelector) -> Option<&'static str> {
    match selector {
        SystemReadSelector::Dmesg => None,
        SystemReadSelector::ProcCmdline => Some("/proc/cmdline"),
        SystemReadSelector::ProcModules => Some("/proc/modules"),
        SystemReadSelector::ProcVersion => Some("/proc/version"),
        SystemReadSelector::ProcMeminfo => Some("/proc/meminfo"),
        SystemReadSelector::ProcLoadavg => Some("/proc/loadavg"),
        SystemReadSelector::ProcUptime => Some("/proc/uptime"),
        SystemReadSelector::ProcCpuinfo => Some("/proc/cpuinfo"),
        SystemReadSelector::ProcInterrupts => Some("/proc/interrupts"),
        SystemReadSelector::ProcSoftirqs => Some("/proc/softirqs"),
        SystemReadSelector::ProcVmstat => Some("/proc/vmstat"),
        SystemReadSelector::ProcBuddyinfo => Some("/proc/buddyinfo"),
        SystemReadSelector::ProcZoneinfo => Some("/proc/zoneinfo"),
        SystemReadSelector::ProcIomem => Some("/proc/iomem"),
        SystemReadSelector::ProcIoports => Some("/proc/ioports"),
        SystemReadSelector::ProcSlabinfo => Some("/proc/slabinfo"),
    }
}

fn read_text_file(path: &str) -> Result<String> {
    fs::read_to_string(path).with_context(|| format!("read {path}"))
}

fn read_json_file(path: PathBuf) -> Result<Value> {
    let raw = fs::read_to_string(&path).with_context(|| format!("read {}", path.display()))?;
    serde_json::from_str(&raw).with_context(|| format!("parse {}", path.display()))
}

fn run_read_command(command: &str, args: &[&str]) -> Result<String> {
    let output = Command::new(command)
        .args(args)
        .output()
        .with_context(|| format!("spawn `{command}`"))?;
    if !output.status.success() {
        bail!(
            "`{command}` failed with status {}",
            output.status.code().unwrap_or_default()
        );
    }
    String::from_utf8(output.stdout).context("command output was not valid UTF-8")
}

impl Default for KernelFacade {
    fn default() -> Self {
        Self::new(DEFAULT_DEBUGFS_ROOT, "/var/log/cilux-broker.log")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cilux_common::{TRACE_EXEC, TRACE_EXIT, TRACE_MODULE, TRACE_OOM};

    #[test]
    fn trace_enable_disable_and_reset_default_mask_math() {
        assert_eq!(
            trace_enable_mask(
                TRACE_EXEC,
                TRACE_DEFAULT_MASK,
                &[TraceCategory::Module, TraceCategory::SchedProcessExec],
            )
            .expect("enable should succeed"),
            TRACE_EXEC | TRACE_MODULE
        );
        assert_eq!(
            trace_disable_mask(
                TRACE_DEFAULT_MASK,
                TRACE_DEFAULT_MASK,
                &[TraceCategory::SchedProcessExit, TraceCategory::MarkVictim],
            )
            .expect("disable should succeed"),
            TRACE_EXEC | TRACE_MODULE
        );
        assert_eq!(
            trace_reset_default_mask(TRACE_EXEC | TRACE_OOM),
            TRACE_EXEC | TRACE_OOM
        );
    }

    #[test]
    fn trace_categories_must_not_be_empty() {
        let error = trace_categories_mask(&[]).expect_err("empty categories should fail");
        assert_eq!(error.to_string(), "trace categories must not be empty");
    }

    #[test]
    fn unsupported_trace_categories_fail() {
        let error = trace_enable_mask(
            TRACE_EXEC,
            TRACE_EXEC | TRACE_EXIT,
            &[TraceCategory::Module],
        )
        .expect_err("unsupported categories should fail");
        assert_eq!(
            error.to_string(),
            "unsupported trace categories requested: module"
        );
    }

    #[test]
    fn system_read_paths_cover_new_selectors() {
        let paths = [
            (
                SystemReadSelector::ProcCmdline,
                system_read_path(SystemReadSelector::ProcCmdline),
            ),
            (
                SystemReadSelector::ProcVersion,
                system_read_path(SystemReadSelector::ProcVersion),
            ),
            (
                SystemReadSelector::ProcSoftirqs,
                system_read_path(SystemReadSelector::ProcSoftirqs),
            ),
            (
                SystemReadSelector::ProcIomem,
                system_read_path(SystemReadSelector::ProcIomem),
            ),
            (
                SystemReadSelector::ProcIoports,
                system_read_path(SystemReadSelector::ProcIoports),
            ),
            (
                SystemReadSelector::ProcSlabinfo,
                system_read_path(SystemReadSelector::ProcSlabinfo),
            ),
        ];

        assert_eq!(
            paths,
            [
                (SystemReadSelector::ProcCmdline, Some("/proc/cmdline")),
                (SystemReadSelector::ProcVersion, Some("/proc/version")),
                (SystemReadSelector::ProcSoftirqs, Some("/proc/softirqs")),
                (SystemReadSelector::ProcIomem, Some("/proc/iomem")),
                (SystemReadSelector::ProcIoports, Some("/proc/ioports")),
                (SystemReadSelector::ProcSlabinfo, Some("/proc/slabinfo")),
            ]
        );
    }
}
