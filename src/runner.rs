use std::path::Path;
use std::process::{Command, Stdio};

use anyhow::Result;

pub struct CmdRunner {
    command: Command,
}

impl CmdRunner {
    pub fn build(cmd_name: &str, args: &[String], dir: impl AsRef<Path>) -> Result<Self> {
        let mut command = Command::new(cmd_name);
        command.current_dir(dir);
        command.args(args);
        command.stdin(Stdio::piped());

        Ok(Self {
            command,
        })
    }

    pub fn run(&mut self) -> Result<()> {
        let mut process = self.command.spawn()?;
        let exit_status = process.wait()?;
        if exit_status.success() {
            Ok(())
        } else {
            anyhow::bail!("process exited with {exit_status}")
        }
    }
}