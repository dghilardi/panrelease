use std::path::PathBuf;
use crate::project::core::PanProject;
use crate::system::FileSystem;
pub struct ConfigLoader;

impl ConfigLoader {
    pub fn parse_config<S: FileSystem>(repo_path: Option<PathBuf>) -> anyhow::Result<PanProject<S>> {
        let Ok(cwd) = repo_path.map(Ok).unwrap_or_else(S::current_dir) else {
            anyhow::bail!("Error loading current directory");
        };

        let project = match PanProject::load(cwd.as_path()) {
            Ok(p) => p,
            Err(e) => {
                anyhow::bail!("Error loading project - {e}");
            }
        };
        Ok(project)
    }
}