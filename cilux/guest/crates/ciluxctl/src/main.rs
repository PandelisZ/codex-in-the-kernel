use anyhow::Result;
use cilux_common::{
    call_broker, pretty_json, BrokerRequest, BufferClearRequest, HealthRequest,
    KernelEventsTailRequest, KernelSnapshotRequest, SystemReadRequest, SystemReadSelector,
    TraceCategoriesRequest, TraceCategory, TraceConfigureRequest, TraceResetDefaultRequest,
    TraceStatusRequest, DEFAULT_BROKER_SOCKET,
};
use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser, Debug)]
struct Args {
    #[arg(long, default_value = DEFAULT_BROKER_SOCKET)]
    socket: PathBuf,
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand, Debug, PartialEq, Eq)]
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
    TraceStatus,
    TraceEnable {
        #[arg(long = "category", required = true)]
        categories: Vec<TraceCategory>,
    },
    TraceDisable {
        #[arg(long = "category", required = true)]
        categories: Vec<TraceCategory>,
    },
    TraceResetDefault,
    BufferClear,
    SystemRead {
        #[arg(long)]
        selector: SystemReadSelector,
    },
}

fn main() -> Result<()> {
    let args = Args::parse();
    let request = broker_request(args.command);

    let response = call_broker(&args.socket, &request)?;
    println!("{}", pretty_json(&response.into_result()?)?);
    Ok(())
}

fn broker_request(command: Command) -> BrokerRequest {
    match command {
        Command::Health => BrokerRequest::Health(HealthRequest::default()),
        Command::Snapshot => BrokerRequest::KernelSnapshot(KernelSnapshotRequest::default()),
        Command::Events { limit } => {
            BrokerRequest::KernelEventsTail(KernelEventsTailRequest { limit })
        }
        Command::TraceConfigure { trace_mask } => {
            BrokerRequest::TraceConfigure(TraceConfigureRequest { trace_mask })
        }
        Command::TraceStatus => BrokerRequest::TraceStatus(TraceStatusRequest::default()),
        Command::TraceEnable { categories } => {
            BrokerRequest::TraceEnable(TraceCategoriesRequest { categories })
        }
        Command::TraceDisable { categories } => {
            BrokerRequest::TraceDisable(TraceCategoriesRequest { categories })
        }
        Command::TraceResetDefault => {
            BrokerRequest::TraceResetDefault(TraceResetDefaultRequest::default())
        }
        Command::BufferClear => BrokerRequest::BufferClear(BufferClearRequest::default()),
        Command::SystemRead { selector } => {
            BrokerRequest::SystemRead(SystemReadRequest { selector })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    #[test]
    fn repeated_trace_categories_parse() {
        let args = Args::try_parse_from([
            "ciluxctl",
            "trace-enable",
            "--category",
            "module",
            "--category",
            "sched_process_exec",
        ])
        .expect("args should parse");

        assert_eq!(
            broker_request(args.command),
            BrokerRequest::TraceEnable(TraceCategoriesRequest {
                categories: vec![TraceCategory::Module, TraceCategory::SchedProcessExec],
            })
        );
    }

    #[test]
    fn new_system_selectors_parse() {
        let parsed = [
            "proc_cmdline",
            "proc_version",
            "proc_softirqs",
            "proc_iomem",
            "proc_ioports",
            "proc_slabinfo",
        ]
        .into_iter()
        .map(|selector| {
            let args = Args::try_parse_from(["ciluxctl", "system-read", "--selector", selector])
                .expect("selector should parse");
            selector
                .parse::<SystemReadSelector>()
                .map(|selector| {
                    broker_request(args.command)
                        == BrokerRequest::SystemRead(SystemReadRequest { selector })
                })
                .expect("selector should round-trip")
        })
        .collect::<Vec<_>>();

        assert_eq!(parsed, vec![true; 6]);
    }
}
