use std::{
    fs::OpenOptions,
    io,
    path::Path,
    process::{Child, Command as StdCommand, Stdio},
    sync::{mpsc, Arc, Mutex},
    thread,
};

use crate::{
    abort::AbortSignal,
    executor::{OnFail, TrackedProcess},
};

pub fn spawn(
    cmd: &str,
    step_index: usize,
    on_fail: OnFail,
    abort_tx: mpsc::Sender<AbortSignal>,
    artifacts_dir: &Path,
) -> io::Result<TrackedProcess> {
    let stdout_path = artifacts_dir.join(format!("step_{:03}_stdout.log", step_index));
    let stderr_path = artifacts_dir.join(format!("step_{:03}_stderr.log", step_index));

    let stdout_file = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(&stdout_path)?;

    let stderr_file = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(&stderr_path)?;

    log::info!("Executing shell command (step {}): {}", step_index, cmd);

    let child = StdCommand::new("sh")
        .arg("-c")
        .arg(cmd)
        .stdout(Stdio::from(stdout_file))
        .stderr(Stdio::from(stderr_file))
        .spawn()?;

    log::info!("Step {} spawned (PID: {:?})", step_index, child.id());

    let child_arc = Arc::new(Mutex::new(Some(child)));
    let child_arc_clone = child_arc.clone();
    let cmd_string = cmd.to_string();

    thread::spawn(move || {
        monitor(child_arc_clone, step_index, on_fail, abort_tx, cmd_string);
    });

    Ok(TrackedProcess { child: child_arc })
}

pub fn monitor(
    child_arc: Arc<Mutex<Option<Child>>>,
    step_index: usize,
    on_fail: OnFail,
    abort_tx: mpsc::Sender<AbortSignal>,
    cmd_string: String,
) {
    loop {
        // Hold the lock only briefly to check status
        let result = {
            let mut child_guard = child_arc.lock().unwrap();
            if let Some(child) = &mut *child_guard {
                child.try_wait()
            } else {
                // Child was taken/killed by stop_all()
                return;
            }
        };

        match result {
            Ok(Some(status)) => {
                // Process has exited
                let exit_code = status.code().unwrap_or(-1);
                log::info!(
                    "Step {} completed with exit code: {}",
                    step_index,
                    exit_code
                );

                if exit_code != 0 && on_fail == OnFail::Abort {
                    log::error!(
                        "Step {} failed with on_fail: Abort, sending abort signal",
                        step_index
                    );
                    let _ = abort_tx.send(AbortSignal {
                        step: step_index,
                        reason: format!(
                            "Process failed with exit code {}: {}",
                            exit_code, cmd_string
                        ),
                    });
                }
                return;
            }
            Ok(None) => {
                // Still running, sleep before checking again
                std::thread::sleep(std::time::Duration::from_millis(100));
            }
            Err(e) => {
                log::error!("Step {} wait error: {}", step_index, e);
                return;
            }
        }
    }
}
