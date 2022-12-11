use std::path::Path;
use anyhow::anyhow;
use git2::{IndexAddOption, IndexEntry, Repository, RepositoryOpenFlags, StatusOptions, StatusShow};
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
            module.hook_after_rel()?;
        }

        self.update_and_commit(new_version)?;

        Ok(())
    }

    fn update_and_commit(&self, version: semver::Version) -> anyhow::Result<()> {
        let mut index = self.repo.index()?;
        index.update_all(["*"].iter(), Some(&mut (|a, b| {
            log::debug!("Adding {:?}", a);
            0
        })))?;
        index.write()?;

        let signature = self.repo.signature()?;
        let oid = index.write_tree()?;
        let tree = self.repo.find_tree(oid)?;
        let parent_commit = self.repo.head()?.peel_to_commit()?;

        let descr = version.to_string();
        let commit_oid = self.repo.commit(Some("HEAD"), &signature, &signature, &descr, &tree, &[&parent_commit])?;

        let commit_obj = self.repo.find_object(commit_oid, None)?;
        self.repo.tag_lightweight(&descr, &commit_obj, false)?;

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