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
}