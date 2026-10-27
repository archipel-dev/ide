//! Running build tools for a resolver: captured output, cancellation, time limit.

use crate::resolver::{CancelToken, ImportError};
use archipel_ide_runner::{TaskPlan, tool_env};
use std::io::Read;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

#[derive(Debug, Clone)]
pub struct Captured {
    pub code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
}

impl Captured {
    pub fn success(&self) -> bool {
        self.code == Some(0)
    }

    /// The last lines of stderr (or stdout when stderr has nothing but JVM notices).
    pub fn tail(&self) -> String {
        let useful = |text: &str| -> Vec<String> {
            text.lines()
                .filter(|line| !line.starts_with("Picked up ") && !line.trim().is_empty())
                .map(str::to_owned)
                .collect()
        };
        let mut lines = useful(&self.stderr);
        if lines.is_empty() {
            lines = useful(&self.stdout);
        }
        lines[lines.len().saturating_sub(12)..].join("\n")
    }
}

/// Output captured per stream is bounded; build tools can be chatty.
const MAX_CAPTURE: usize = 8 * 1024 * 1024;

fn drain(mut reader: impl Read + Send + 'static) -> std::thread::JoinHandle<String> {
    std::thread::spawn(move || {
        let mut bytes = Vec::new();
        let mut chunk = [0_u8; 8192];
        loop {
            match reader.read(&mut chunk) {
                Ok(0) | Err(_) => break,
                Ok(n) => {
                    if bytes.len() < MAX_CAPTURE {
                        bytes.extend_from_slice(&chunk[..n]);
                    }
                }
            }
        }
        String::from_utf8_lossy(&bytes).into_owned()
    })
}

/// Runs `plan` to completion. The managed tool environment (managed JDK, Gradle, Maven on
/// `PATH`, `JAVA_HOME`) applies, exactly as for tasks run from the Build tool window.
pub fn run_plan(
    plan: &TaskPlan,
    cancel: &CancelToken,
    time_limit: Duration,
) -> Result<Captured, ImportError> {
    cancel.check()?;
    let mut command = Command::new(&plan.program);
    tool_env::tool_environment().apply(&mut command, &plan.env);
    command
        .args(&plan.args)
        .current_dir(&plan.cwd)
        .envs(plan.env.iter().map(|(k, v)| (k, v)))
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = command.spawn().map_err(|error| {
        if error.kind() == std::io::ErrorKind::NotFound {
            ImportError::ToolMissing(format!("`{}` was not found", plan.program))
        } else {
            ImportError::Io(format!("could not start {}: {error}", plan.program))
        }
    })?;
    let stdout = child.stdout.take().map(drain);
    let stderr = child.stderr.take().map(drain);
    let started = Instant::now();
    let status = loop {
        if cancel.is_cancelled() {
            let _ = child.kill();
            let _ = child.wait();
            return Err(ImportError::Cancelled);
        }
        if started.elapsed() > time_limit {
            let _ = child.kill();
            let _ = child.wait();
            return Err(ImportError::ToolFailed {
                tool: plan.program.clone(),
                message: format!("timed out after {} s", time_limit.as_secs()),
            });
        }
        match child.try_wait()? {
            Some(status) => break status,
            None => std::thread::sleep(Duration::from_millis(40)),
        }
    };
    let join = |handle: Option<std::thread::JoinHandle<String>>| {
        handle.and_then(|h| h.join().ok()).unwrap_or_default()
    };
    Ok(Captured {
        code: status.code(),
        stdout: join(stdout),
        stderr: join(stderr),
    })
}
