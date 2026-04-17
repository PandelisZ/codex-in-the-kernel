mod catalog;
mod json_rpc;
mod resources;
mod tools;

use anyhow::Result;
use cilux_common::DEFAULT_BROKER_SOCKET;
use clap::Parser;
use std::io;
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
    json_rpc::serve(&args.socket, stdin.lock(), stdout.lock())
}
