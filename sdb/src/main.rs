use anyhow::{Context, Result, bail};
use clap::Parser;
use env_logger::Env;

use nix::sys::ptrace;
use nix::unistd::{self, Pid};

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg()]
    program_path: String,

    #[arg(short = 'p')]
    pid: Option<i32>,
}

fn attach(args: &Args) -> Result<Pid> {
    match args.pid {
        Some(pid) => {
            if pid <= 0 {
                bail!("Invalid pid")
            }
            let pid = Pid::from_raw(pid);
            ptrace::attach(pid).with_context(|| format!("failed to attach process {}", pid))?;
        }
        None => {
            let fork_result = unsafe { unistd::fork().context("fork failed")? };
            if fork_result.is_child() {
                trace
            }
        }
    }

    // placeholder
    Pid::from_raw(args.pid.unwrap_or(-1))
}

fn main() -> Result<()> {
    let mut builder = env_logger::Builder::from_env(Env::default().default_filter_or("info"));
    builder.target(env_logger::Target::Stdout);
    builder.init();

    let args = Args::parse();
    attach(&args);

    Ok(())
}
