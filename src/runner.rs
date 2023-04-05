use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus, Output, Stdio};

use anyhow::{anyhow, Result};
use serde_json::json;
use wasm_bindgen::JsValue;
use crate::wasm_utils::exec;

#[cfg(not(target_arch = "wasm32"))]
pub struct CmdRunner {
    command: Command,
}

#[cfg(not(target_arch = "wasm32"))]
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

    pub fn output(&mut self) -> Result<Vec<u8>> {
        let out = self.command.output()?;
        let exit_status = out.status;
        if exit_status.success() {
            Ok(out.stdout)
        } else {
            anyhow::bail!("process exited with {out:?}")
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
            .expect("Error serializing options");
        let out = exec(self.command.clone(), opts)
            .map_err(|e| anyhow!("Error executing command - {e:?}"))
            .and_then(|res| Ok(String::from_utf8(res)?))?;
        crate::wasm_utils::log(&out);
        Ok(())
    }

    pub fn output(&mut self) -> Result<Vec<u8>> {
        let opts = serde_wasm_bindgen::to_value(&json!({ "cwd": &self.dir }))
            .expect("Error serializing options");
        let out = exec(self.command.clone(), opts)
            .map_err(|e| anyhow!("Error executing command - {e:?}"))?;
        Ok(out)
    }
}