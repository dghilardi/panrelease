use std::process::{Command, Stdio};

use anyhow::Result;

pub struct CmdRunner {
    command: Command,
}

impl CmdRunner {
    pub fn build(cmd_name: &str, args: &[String]) -> Result<Self> {
        let mut command = Command::new(cmd_name);
        command.args(args);
        command.stdin(Stdio::piped());

        Ok(Self {
            command,
        })
    }

    pub fn run(&mut self) -> Result<()> {
        let mut process = self.command.spawn()?;
        process.wait()?;
        Ok(())
    }
}