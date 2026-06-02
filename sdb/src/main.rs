use anyhow::{Context, Result, bail};
use clap::{Parser, Subcommand};
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

#[derive(Parser, Debug)]
#[command(multicall = true)]
enum DebuggerCommands {
    #[command(visible_aliases = ["c", "cont"])]
    Continue,
    #[command(subcommand)]
    Break(BreakCommand),
}

#[derive(Subcommand, Debug)]
enum BreakCommand {
    Set { address: String },
}

impl From<Args> for libsdb::process::AttachTarget {
    fn from(args: Args) -> Self {
        if let Some(pid) = args.pid {
            return libsdb::process::AttachTarget::Pid(pid);
        }
        if let Some(program) = args.program {
            return libsdb::process::AttachTarget::Program(program);
        }

        unreachable!(
            "If this ends up broken it means the clap argument parser implementation is broken."
        );
    }
}

fn handle_raw_command(pid: Pid, raw_command: Option<Vec<String>>) -> Result<()> {
    let Some(arguments) = raw_command else {
        return Ok(());
    };

    match DebuggerCommands::try_parse_from(arguments) {
        Ok(command) => match command {
            DebuggerCommands::Continue => {
                println!("User typed a continue command");
                // resume(pid).context("continue a process")?;
                // wait_on_signal(pid).context("waiting for paused process to continue")?
            }
            DebuggerCommands::Break(BreakCommand::Set { address }) => {
                println!("User typed a break command with address {}", address);
            }
        },
        Err(err) => log::info!("{}", "provided an unknown command"),
    }
    Ok(())
}

fn main() -> Result<()> {
    let mut builder = env_logger::Builder::from_env(Env::default().default_filter_or("info"));
    builder.target(env_logger::Target::Stdout);
    builder.init();

    let target: libsdb::process::AttachTarget = Args::parse().into();
    let process = libsdb::process::Process::new(target)?;

    let mut editor = DefaultEditor::new().context("Creates a command line interface")?;

    loop {
        let read_line = editor.readline("sdb> ");
        match read_line {
            Ok(line) => {
                editor
                    .add_history_entry(line.as_str())
                    .context("adding to shell history")?;
                handle_raw_command(process.pid, shlex::split(&line))?;
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
