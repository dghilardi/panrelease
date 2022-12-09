use std::path::Path;
use anyhow::anyhow;
use git2::{Repository, RepositoryOpenFlags, StatusOptions};
use crate::project::config::PanProjectConfig;

pub struct PanProject {
    conf: PanProjectConfig,
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
            conf,
            repo,
        })
    }

    pub fn release(&self) -> anyhow::Result<()> {
        if !self.repo.statuses(None)?.is_empty() {
            return Err(anyhow!("Repository status is not clean"));
        }
        Ok(())
    }
}