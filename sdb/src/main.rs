use anyhow::{Context, Result, bail};
use clap::Parser;
use env_logger::Env;

use rustyline::DefaultEditor;
use rustyline::error::ReadlineError;
use rustyline::history::History;

// Rust-idiomatic cross platform wrapper to Unix-like system APIs
use nix::sys;
use nix::unistd::{self, Pid};

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
#[group(id = "target", required = true, multiple = false)]
struct Args {
    #[arg(group = "target")]
    program: Option<String>,

    #[arg(short = 'p', group = "target")]
    pid: Option<i32>,
}

enum AttachTarget {
    Pid(i32),
    Program(String),
}

impl From<Args> for AttachTarget {
    fn from(args: Args) -> Self {
        if let Some(pid) = args.pid {
            return AttachTarget::Pid(pid);
        }
        if let Some(program) = args.program {
            return AttachTarget::Program(program);
        }

        unreachable!(
            "If this ends up broken it means the clap argument parser implementation is broken."
        );
    }
}

fn attach(target: AttachTarget) -> Result<Pid> {
    match target {
        AttachTarget::Pid(pid) => {
            if pid <= 0 {
                bail!("Invalid pid")
            }
            let pid = Pid::from_raw(pid);
            sys::ptrace::attach(pid).with_context(|| format!("attach to process {}", pid))?;
            Ok(pid)
        }
        AttachTarget::Program(program) => {
            let fork_result = unsafe { unistd::fork().context("fork the program")? };
            if fork_result.is_child() {
                sys::ptrace::traceme()
                    .context("allow to send more ptrace request to this process in the future")?;
                let program_path = std::ffi::CString::new(program)
                    .context("exec_vector_path requires a c-string")?;

                unistd::execvp(&program_path, &[&program_path])?;
            }
            Ok(Pid::from_raw(0))
        }
    }
}

fn handle_command(pid: Pid, line: &str) {
    println!("Called code {}", line);
}

fn main() -> Result<()> {
    let mut builder = env_logger::Builder::from_env(Env::default().default_filter_or("info"));
    builder.target(env_logger::Target::Stdout);
    builder.init();

    let target: AttachTarget = Args::parse().into();
    let pid = attach(target).context("Attaching to a process")?;
    let wait_status = sys::wait::waitpid(pid, None)
        .context("wait for child process to change status / has child changed status")?;

    let mut editor = DefaultEditor::new()?;

    loop {
        let read_line = editor.readline("sdb> ");
        match read_line {
            Ok(line) => {
                editor
                    .add_history_entry(line.as_str())
                    .context("adding to shell history")?;
                handle_command(pid, line.as_str());
            }
            Err(error) => {
                match error {
                    ReadlineError::Eof | ReadlineError::Interrupted => {
                        log::info!("debugger was interrupted by the user")
                    }
                    _ => log::warn!("failed to read command, error: {:?}", error),
                }

                break;
            }
        };
    }
    Ok(())
}
