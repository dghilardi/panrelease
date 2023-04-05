use std::collections::HashMap;
use std::path::{Path, PathBuf};
use anyhow::Result;

pub trait EnvVars {
    fn prefixed(prefix: &str) -> Result<HashMap<String, String>>;
}

pub trait FileSystem {
    fn read_string(path: &str) -> Result<String>;
    fn write_string(path: &str, content: &str) -> Result<()>;
    fn current_dir() -> Result<PathBuf>;
    fn is_a_dir(path: &Path) -> bool;
    fn is_a_file(path: &Path) -> bool;
}

