use std::time::Duration;
use tokio::process::Command;
use tokio::time::timeout;

#[derive(Debug, Clone)]
pub struct CommandOutput {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i32,
    pub timed_out: bool,
}

#[derive(Debug, Clone, Default)]
pub struct CommandRunner;

impl CommandRunner {
    pub async fn run(
        &self,
        program: &str,
        args: &[&str],
        timeout_secs: Option<u64>,
    ) -> CommandOutput {
        let timeout_duration = Duration::from_secs(timeout_secs.unwrap_or(30));

        let child = match Command::new(program)
            .args(args)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
        {
            Ok(c) => c,
            Err(e) => {
                return CommandOutput {
                    stdout: String::new(),
                    stderr: e.to_string(),
                    exit_code: -1,
                    timed_out: false,
                };
            }
        };

        let output_future = child.wait_with_output();
        let result = timeout(timeout_duration, output_future).await;

        match result {
            Ok(Ok(output)) => {
                CommandOutput {
                    stdout: String::from_utf8_lossy(&output.stdout).to_string(),
                    stderr: String::from_utf8_lossy(&output.stderr).to_string(),
                    exit_code: output.status.code().unwrap_or(-1),
                    timed_out: false,
                }
            }
            Ok(Err(e)) => CommandOutput {
                stdout: String::new(),
                stderr: e.to_string(),
                exit_code: -1,
                timed_out: false,
            },
            Err(_) => {
                CommandOutput {
                    stdout: String::new(),
                    stderr: "command timed out".to_string(),
                    exit_code: -1,
                    timed_out: true,
                }
            }
        }
    }

    pub fn run_stub(&self, command: &str) -> CommandOutput {
        CommandOutput {
            stdout: format!("stubbed command: {command}"),
            stderr: String::new(),
            exit_code: 0,
            timed_out: false,
        }
    }
}
