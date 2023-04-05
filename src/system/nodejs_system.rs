use std::collections::HashMap;
use std::path::{Path, PathBuf};
use anyhow::anyhow;
use crate::system::contract::{EnvVars, FileSystem};
use crate::wasm_utils;


pub struct NodeJsSystem;

impl EnvVars for NodeJsSystem {
    fn prefixed(prefix: &str) -> anyhow::Result<HashMap<String, String>> {
        let full_env: HashMap<String, String> = wasm_utils::process::ENV.into_serde()?;
        let filtered_env = full_env.into_iter()
            .filter_map(|(key, value)|
                key
                    .strip_prefix(prefix)
                    .map(|subkey| (subkey.to_string(), value))
            )
            .collect();
        Ok(filtered_env)
    }
}

impl FileSystem for NodeJsSystem {
    fn read_string(path: &str) -> anyhow::Result<String> {
        let content = wasm_utils::read_file(path, "utf8")
            .map_err(|e| anyhow!("Error reading file - {e:?}"))?;
        Ok(content)
    }

    fn write_string(path: &str, content: &str) -> anyhow::Result<()> {
        wasm_utils::write_file(path, content)
            .map_err(|e| anyhow!("Error writing file - {e:?}"))?;
        Ok(())
    }

    fn current_dir() -> anyhow::Result<PathBuf> {
        let cwd = PathBuf::from(wasm_utils::process::cwd());
        Ok(cwd)
    }

    fn is_a_dir(path: &Path) -> bool {
        wasm_utils::exists(path.to_str().expect("Invalid path"))
            .expect("Error checking file existence")
    }

    fn is_a_file(path: &Path) -> bool {
        wasm_utils::exists(path.to_str().expect("Invalid path"))
            .expect("Error checking file existence")
    }
}