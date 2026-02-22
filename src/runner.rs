use std::path::Path;
#[cfg(not(target_arch = "wasm32"))]
use std::process::{Command, Stdio};
#[cfg(not(target_arch = "wasm32"))]
use std::io;

use anyhow::Result;

#[cfg(target_arch = "wasm32")]
use std::path::PathBuf;
#[cfg(target_arch = "wasm32")]
use anyhow::anyhow;
#[cfg(target_arch = "wasm32")]
use serde_json::json;
#[cfg(target_arch = "wasm32")]
use crate::wasm_utils::exec;

#[cfg(not(target_arch = "wasm32"))]
pub struct CmdRunner {
    cmd_name: String,
    cmd_display: String,
    cwd: std::path::PathBuf,
    command: Command,
}

#[cfg(not(target_arch = "wasm32"))]
impl CmdRunner {
    pub fn build(cmd_name: &str, args: &[String], dir: impl AsRef<Path>) -> Result<Self> {
        let cwd = dir.as_ref().to_path_buf();
        let mut command = Command::new(cmd_name);
        command.current_dir(&cwd);
        command.args(args);
        command.stdin(Stdio::piped());

        Ok(Self {
            cmd_name: cmd_name.to_string(),
            cmd_display: format!("{cmd_name} {}", args.join(" ")),
            cwd,
            command,
        })
    }

    fn not_found_error(&self) -> anyhow::Error {
        // Distinguish "command not in PATH" from "working directory does not exist".
        // Both produce ErrorKind::NotFound on Linux, so we check the cwd explicitly.
        if !self.cwd.exists() {
            anyhow::anyhow!(
                "failed to run `{}`: working directory {:?} does not exist",
                self.cmd_display, self.cwd
            )
        } else {
            anyhow::anyhow!(
                "command not found: `{}` — is it installed and in your PATH?",
                self.cmd_name
            )
        }
    }

    pub fn run(&mut self) -> Result<()> {
        let mut process = self.command.spawn()
            .map_err(|e| if e.kind() == io::ErrorKind::NotFound {
                self.not_found_error()
            } else {
                anyhow::anyhow!("failed to spawn `{}`: {e}", self.cmd_display)
            })?;
        let exit_status = process.wait()?;
        if exit_status.success() {
            Ok(())
        } else {
            anyhow::bail!(
                "command `{}` failed with {}",
                self.cmd_display, exit_status
            )
        }
    }

    pub fn output(&mut self) -> Result<Vec<u8>> {
        let out = self.command.output()
            .map_err(|e| if e.kind() == io::ErrorKind::NotFound {
                self.not_found_error()
            } else {
                anyhow::anyhow!("failed to run `{}`: {e}", self.cmd_display)
            })?;
        if out.status.success() {
            Ok(out.stdout)
        } else {
            let stderr = String::from_utf8_lossy(&out.stderr);
            let stderr = stderr.trim();
            if stderr.is_empty() {
                anyhow::bail!(
                    "command `{}` failed with {}",
                    self.cmd_display, out.status
                )
            } else {
                anyhow::bail!(
                    "command `{}` failed with {}:\n{}",
                    self.cmd_display, out.status, stderr
                )
            }
        }
    }
}

#[cfg(target_arch = "wasm32")]
pub struct CmdRunner {
    command: String,
    dir: PathBuf
}

#[cfg(target_arch = "wasm32")]
impl CmdRunner {
    pub fn build(cmd_name: &str, args: &[String], dir: impl AsRef<Path>) -> Result<Self> {
        Ok(Self {
            command: format!("{cmd_name} {}", args.join(" ")),
            dir: dir.as_ref().to_path_buf(),
        })
    }

    pub fn run(&mut self) -> Result<()> {
        let opts = serde_wasm_bindgen::to_value(&json!({ "cwd": &self.dir }))
            .map_err(|e| anyhow!("Error serializing command options: {e:?}"))?;
        let out = exec(self.command.clone(), opts)
            .map_err(|e| anyhow!("Error executing command `{}`: {e:?}", self.command))
            .and_then(|res| Ok(String::from_utf8(res)?))?;
        crate::wasm_utils::log(&out);
        Ok(())
    }

    pub fn output(&mut self) -> Result<Vec<u8>> {
        let opts = serde_wasm_bindgen::to_value(&json!({ "cwd": &self.dir }))
            .map_err(|e| anyhow!("Error serializing command options: {e:?}"))?;
        let out = exec(self.command.clone(), opts)
            .map_err(|e| anyhow!("Error executing command `{}`: {e:?}", self.command))?;
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn run_failing_command_includes_command_name_in_error() {
        let mut runner = CmdRunner::build(
            "sh",
            &["-c".to_string(), "exit 42".to_string()],
            ".",
        ).unwrap();
        let err = runner.run().unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("sh"), "error should mention command name: {msg}");
        assert!(msg.contains("42"), "error should mention exit code: {msg}");
    }

    #[test]
    fn output_failing_command_includes_stderr_in_error() {
        let mut runner = CmdRunner::build(
            "sh",
            &["-c".to_string(), "echo 'something went wrong' >&2; exit 1".to_string()],
            ".",
        ).unwrap();
        let err = runner.output().unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("something went wrong"), "error should include stderr: {msg}");
    }

    #[test]
    fn run_successful_command_returns_ok() {
        let mut runner = CmdRunner::build(
            "sh",
            &["-c".to_string(), "exit 0".to_string()],
            ".",
        ).unwrap();
        assert!(runner.run().is_ok());
    }

    #[test]
    fn run_nonexistent_command_mentions_not_found_and_path() {
        let mut runner = CmdRunner::build(
            "this_command_does_not_exist_panrelease",
            &[],
            ".",
        ).unwrap();
        let err = runner.run().unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("not found"), "error should say command was not found: {msg}");
        assert!(
            msg.contains("this_command_does_not_exist_panrelease"),
            "error should mention the command name: {msg}"
        );
        assert!(msg.contains("PATH"), "error should mention PATH: {msg}");
    }

    #[test]
    fn output_nonexistent_command_mentions_not_found_and_path() {
        let mut runner = CmdRunner::build(
            "this_command_does_not_exist_panrelease",
            &[],
            ".",
        ).unwrap();
        let err = runner.output().unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("not found"), "error should say command was not found: {msg}");
        assert!(msg.contains("PATH"), "error should mention PATH: {msg}");
    }

    #[test]
    fn run_nonexistent_cwd_does_not_blame_missing_command() {
        let mut runner = CmdRunner::build(
            "git",
            &["status".to_string()],
            "/tmp/this_cwd_does_not_exist_panrelease",
        ).unwrap();
        let err = runner.run().unwrap_err();
        let msg = err.to_string();
        // Should mention the directory, NOT suggest installing git
        assert!(
            msg.contains("this_cwd_does_not_exist_panrelease"),
            "error should mention the bad cwd: {msg}"
        );
        assert!(
            !msg.contains("installed and in your PATH"),
            "error should NOT blame a missing command when cwd doesn't exist: {msg}"
        );
    }
}
