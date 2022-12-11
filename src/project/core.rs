use std::path::Path;
use anyhow::anyhow;
use git2::{Repository, RepositoryOpenFlags, StatusOptions, StatusShow};
use semver::Prerelease;
use crate::args::RelArgs;
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

    pub fn release(&self, rel_args: RelArgs) -> anyhow::Result<()> {
        let mut opts = StatusOptions::new();
        opts
            .include_unmodified(false)
            .include_untracked(false)
            .include_ignored(false);

        if !self.repo.statuses(Some(&mut opts))?.is_empty() {
            return Err(anyhow!("Repository status is not clean"));
        }
        let new_version = rel_args.level_or_version.apply(self.extract_version()?);
        for mut module in self.conf.modules()? {
            module.set_version(&new_version)?;
            module.persist()?;
        }
        Ok(())
    }

    fn extract_version(&self) -> anyhow::Result<semver::Version> {
        let maybe_module = self.conf.extract_master_mod()?;
        if let Some(module) = maybe_module {
            module.extract_version()
        } else {
            todo!("Module detection not yet implemented")
        }
    }
}