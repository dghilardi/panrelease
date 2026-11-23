use std::collections::HashMap;
use std::path::{Path, PathBuf};
use anyhow::anyhow;
use crate::system::contract::{EnvVars, FileSystem};
use crate::wasm_utils;

#[derive(Default)]
pub struct NodeJsSystem;

impl EnvVars for NodeJsSystem {
    fn prefixed(prefix: &str) -> anyhow::Result<HashMap<String, String>> {
        let full_env: HashMap<String, String> = serde_wasm_bindgen::from_value(wasm_utils::process::ENV.clone())
            .map_err(|e| anyhow!("Error deserializing env: {e:?}"))?;
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
    fn read_string(path: &Path) -> anyhow::Result<String> {
        let path_str = path.to_str()
            .ok_or_else(|| anyhow!("Path is not valid UTF-8: {:?}", path))?;
        let content = wasm_utils::read_file(path_str, "utf8")
            .map_err(|e| anyhow!("Error reading file {:?}: {e:?}", path))?;
        Ok(content)
    }

    fn write_string(path: &Path, content: &str) -> anyhow::Result<()> {
        let path_str = path.to_str()
            .ok_or_else(|| anyhow!("Path is not valid UTF-8: {:?}", path))?;
        wasm_utils::write_file(path_str, content)
            .map_err(|e| anyhow!("Error writing file {:?}: {e:?}", path))?;
        Ok(())
    }

    fn current_dir() -> anyhow::Result<PathBuf> {
        let cwd = PathBuf::from(wasm_utils::process::cwd());
        Ok(cwd)
    }

    fn is_a_dir(path: &Path) -> bool {
        let Some(path_str) = path.to_str() else { return false; };
        wasm_utils::exists(path_str).unwrap_or(false)
    }

    fn is_a_file(path: &Path) -> bool {
        let Some(path_str) = path.to_str() else { return false; };
        wasm_utils::exists(path_str).unwrap_or(false)
    }

    fn create_dir_all(path: &Path) -> anyhow::Result<()> {
        let path_str = path.to_str()
            .ok_or_else(|| anyhow!("Path is not valid UTF-8: {:?}", path))?;
        let opts = serde_wasm_bindgen::to_value(&serde_json::json!({ "recursive": true }))
            .map_err(|e| anyhow!("Error serializing mkdir options: {e:?}"))?;
        wasm_utils::mkdir(path_str, opts)
            .map_err(|e| anyhow!("Error creating directory {:?}: {e:?}", path))?;
        Ok(())
    }

    fn home_dir() -> anyhow::Result<PathBuf> {
        Ok(PathBuf::from(wasm_utils::os::homedir()))
    }
}
