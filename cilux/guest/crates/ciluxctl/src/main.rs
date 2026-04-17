use anyhow::Result;
use cilux_common::{
    call_broker, pretty_json, BrokerRequest, BufferClearRequest, HealthRequest,
    KernelEventsTailRequest, KernelSnapshotRequest, SystemReadRequest, SystemReadSelector,
    TraceConfigureRequest, DEFAULT_BROKER_SOCKET,
};
use clap::{Parser, Subcommand, ValueEnum};
use std::path::PathBuf;

#[derive(Parser, Debug)]
struct Args {
    #[arg(long, default_value = DEFAULT_BROKER_SOCKET)]
    socket: PathBuf,
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    Health,
    Snapshot,
    Events {
        #[arg(long, default_value_t = 32)]
        limit: usize,
    },
    TraceConfigure {
        #[arg(long)]
        trace_mask: u32,
    },
    BufferClear,
    SystemRead {
        #[arg(long)]
        selector: Selector,
    },
}

#[derive(Clone, Debug, ValueEnum)]
enum Selector {
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

impl From<Selector> for SystemReadSelector {
    fn from(value: Selector) -> Self {
        match value {
            Selector::Dmesg => Self::Dmesg,
            Selector::ProcModules => Self::ProcModules,
            Selector::ProcMeminfo => Self::ProcMeminfo,
            Selector::ProcLoadavg => Self::ProcLoadavg,
            Selector::ProcUptime => Self::ProcUptime,
            Selector::ProcCpuinfo => Self::ProcCpuinfo,
            Selector::ProcInterrupts => Self::ProcInterrupts,
            Selector::ProcVmstat => Self::ProcVmstat,
            Selector::ProcBuddyinfo => Self::ProcBuddyinfo,
            Selector::ProcZoneinfo => Self::ProcZoneinfo,
        }
    }
}

fn main() -> Result<()> {
    let args = Args::parse();
    let request = match args.command {
        Command::Health => BrokerRequest::Health(HealthRequest::default()),
        Command::Snapshot => BrokerRequest::KernelSnapshot(KernelSnapshotRequest::default()),
        Command::Events { limit } => {
            BrokerRequest::KernelEventsTail(KernelEventsTailRequest { limit })
        }
        Command::TraceConfigure { trace_mask } => {
            BrokerRequest::TraceConfigure(TraceConfigureRequest { trace_mask })
        }
        Command::BufferClear => BrokerRequest::BufferClear(BufferClearRequest::default()),
        Command::SystemRead { selector } => BrokerRequest::SystemRead(SystemReadRequest {
            selector: selector.into(),
        }),
    };

    let response = call_broker(&args.socket, &request)?;
    println!("{}", pretty_json(&response.into_result()?)?);
    Ok(())
}
