use crate::netlink;
use anyhow::{bail, Context, Result};
use cilux_common::{
    HealthReport, KernelEventsTail, KernelSnapshot, StatusResult, SystemReadResult,
    SystemReadSelector, TraceConfigureResult, DEFAULT_APP_SERVER_PORT, DEFAULT_DEBUGFS_ROOT,
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
        let text = match selector {
            SystemReadSelector::Dmesg => run_read_command("dmesg", &[])?,
            SystemReadSelector::ProcModules => read_proc_file("/proc/modules")?,
            SystemReadSelector::ProcMeminfo => read_proc_file("/proc/meminfo")?,
            SystemReadSelector::ProcLoadavg => read_proc_file("/proc/loadavg")?,
            SystemReadSelector::ProcUptime => read_proc_file("/proc/uptime")?,
            SystemReadSelector::ProcCpuinfo => read_proc_file("/proc/cpuinfo")?,
            SystemReadSelector::ProcInterrupts => read_proc_file("/proc/interrupts")?,
            SystemReadSelector::ProcVmstat => read_proc_file("/proc/vmstat")?,
            SystemReadSelector::ProcBuddyinfo => read_proc_file("/proc/buddyinfo")?,
            SystemReadSelector::ProcZoneinfo => read_proc_file("/proc/zoneinfo")?,
        };

        Ok(SystemReadResult { selector, text })
    }
}

fn read_proc_file(path: &str) -> Result<String> {
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
