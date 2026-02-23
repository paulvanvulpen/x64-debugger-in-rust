use anyhow::{Context, Result, bail};
use clap::Parser;
use env_logger::Env;

use nix::sys;
use nix::unistd::{self, Pid};

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg()]
    program_path: String,

    #[arg(short = 'p')]
    pid: Option<i32>,
}

fn attach(args: Args) -> Result<Pid> {
    match args.pid {
        Some(pid) => {
            if pid <= 0 {
                bail!("Invalid pid")
            }
            let pid = Pid::from_raw(pid);
            sys::ptrace::attach(pid).with_context(|| format!("attach to process {}", pid))?;
            Ok(pid)
        }
        None => {
            let fork_result = unsafe { unistd::fork().context("fork failed")? };
            if fork_result.is_child() {
                sys::ptrace::traceme()
                    .context("allow to send more ptrace request to this process in the future")?;
                let program_path = std::ffi::CString::new(args.program_path)
                    .context("exec_vector_path requires a c-string")?;

                unistd::execvp(&program_path, &[&program_path])?;
                let wait_status = sys::wait::wait().context(
                    "wait for child process to change status / has child changed status",
                )?;
            }
            Ok(Pid::from_raw(0))
        }
    }
}

fn main() -> Result<()> {
    let mut builder = env_logger::Builder::from_env(Env::default().default_filter_or("info"));
    builder.target(env_logger::Target::Stdout);
    builder.init();

    let args = Args::parse();
    attach(args);

    Ok(())
}
