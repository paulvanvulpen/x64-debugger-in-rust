use anyhow::{Context, Result, bail};
use nix::sys;
use nix::unistd::{self, Pid};
use std::path::Path;

enum ProcessState {
    Stopped,
    Running,
    Exited,
    Terminated,
}

pub struct Process {
    pid: Pid,
    terminate_on_end: bool,
    pub state: ProcessState,
}

impl Process {
    /// Launches a new child process in a traced state using `ptrace`.
    ///
    /// This function forks the current process. The child process enables tracing on itself
    /// and executes the target program, while the parent waits for the initial trap signal.
    ///
    /// ### References
    /// * **`ptrace(2)`:** [Linux Man Pages](https://man7.org/linux/man-pages/man2/ptrace.2.html)
    ///   > Indicate that this process is to be traced by its parent.
    /// * **`execve(2)`:** [Linux Man Pages](https://man7.org/linux/man-pages/man2/execve.2.html)
    ///   > If the current program is being ptraced, a SIGTRAP signal is sent to it after a successful execve().
    pub fn launch(path: String) -> Result<Self> {
        match unsafe { unistd::fork().context("fork the program")? } {
            // now there are two identical copies of this code running, so we need to catch who is the child
            unistd::ForkResult::Child => {
                sys::ptrace::traceme()
                    .context("allow to send more ptrace request to this process in the future")?;
                let program_path =
                    std::ffi::CString::new(path).context("exec_vector_path requires a c-string")?;

                let error = unistd::execvp(&program_path, &[&program_path]).unwrap_err();

                // unless an error occurs the code below is never executed by the child process.
                eprintln!("Child failed to exec program: {:?}", error);
                std::process::exit(1);
            }
            unistd::ForkResult::Parent { child } => {
                let child_process = Self {
                    pid: child,
                    terminate_on_end: true,
                    state: ProcessState::Stopped,
                };

                child_process
                    .wait_on_signal()
                    .context("Waiting for the child process to halt")?;
                Ok(child_process)
            }
        }
    }

    /// Attaches to an actively running process by its Process ID (PID).
    ///
    /// This sends a request to the target process making it a tracee of the current process.
    ///
    /// ### References
    /// * **`ptrace(2)` Attach:** [Linux Man Pages](https://man7.org/linux/man-pages/man2/ptrace.2.html)
    ///   > Attach to the process specified in pid, making it a tracee of the calling process.
    ///   > The tracee is sent a SIGSTOP, but will not necessarily have stopped by the completion of this call; use waitpid(2) to wait for the tracee to stop.
    pub fn attach(raw_pid: i32) -> Result<Self> {
        if raw_pid <= 0 {
            bail!("Invalid pid")
        }
        let pid = Pid::from_raw(raw_pid);
        sys::ptrace::attach(pid).with_context(|| format!("attach to process {}", pid))?;

        let attached_process = Self {
            pid,
            terminate_on_end: false,
            state: ProcessState::Stopped,
        };

        attached_process
            .wait_on_signal()
            .context("waiting for the attached process to halt")?;
        Ok(attached_process)
    }

    fn resume(pid: Pid) -> Result<()> {
        sys::ptrace::cont(pid.clone(), None)?;
        Ok(())
    }

    fn wait_on_signal(&self) -> Result<()> {
        sys::wait::waitpid(self.pid, None)?;
        Ok(())
    }
}

impl Drop for Process {
    fn drop(&mut self) {
        if self.terminate_on_end {
            todo!("implement what to do")
        } else {
            todo!("implement what to do")
        }
    }
}
