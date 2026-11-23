use std::path::{Path, PathBuf};

use crate::system::contract::FileSystem;

#[derive(Default)]
pub struct NativeSystem;

impl FileSystem for NativeSystem {
    fn read_string(path: &Path) -> anyhow::Result<String> {
        let content = std::fs::read_to_string(path)?;
        Ok(content)
    }

    fn write_string(path: &Path, content: &str) -> anyhow::Result<()> {
        std::fs::write(path, content)?;
        Ok(())
    }

    fn current_dir() -> anyhow::Result<PathBuf> {
        Ok(std::env::current_dir()?)
    }

    fn is_a_dir(path: &Path) -> bool {
        path.is_dir()
    }

    fn is_a_file(path: &Path) -> bool {
        path.is_file()
    }

    fn create_dir_all(path: &Path) -> anyhow::Result<()> {
        std::fs::create_dir_all(path)?;
        Ok(())
    }

    fn home_dir() -> anyhow::Result<PathBuf> {
        std::env::var_os("HOME")
            .or_else(|| std::env::var_os("USERPROFILE"))
            .filter(|home| !home.is_empty())
            .map(PathBuf::from)
            .ok_or_else(|| anyhow::anyhow!("Could not determine the home directory: HOME is not set"))
    }
}