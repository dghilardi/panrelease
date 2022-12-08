use std::path::Path;
use anyhow::anyhow;
use git2::{Repository, RepositoryOpenFlags};
use crate::project::config::PanProjectConfig;

pub struct PanProject {
    repo: Repository,
}

impl PanProject {
    pub fn load(path: &Path) -> anyhow::Result<Self> {
        let repo = Repository::open_ext(
            path,
            RepositoryOpenFlags::empty(),
            &[path]
        )?;
        let project_root = repo.path().parent()
            .ok_or(anyhow!("Error extracting project path from repo"))?;
        let conf = PanProjectConfig::load(project_root)?;

        Ok(Self {
            repo
        })
    }
}