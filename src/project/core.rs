use std::path::{Path, PathBuf};
use anyhow::anyhow;
use git2::{IndexAddOption, IndexEntry, Repository, RepositoryOpenFlags, StatusOptions, StatusShow};
use semver::Prerelease;
use crate::args::RelArgs;
use crate::project::config::PanProjectConfig;
use crate::project::module::PanModule;

pub struct PanProject {
    path: PathBuf,
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
            path: path.to_path_buf(),
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
        let new_version = rel_args.level_or_version.apply(self.extract_master()?.extract_version()?);
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

    fn extract_modules(&self) -> anyhow::Result<Vec<PanModule>> {
        let modules = self.conf.modules()?;
        if modules.is_empty() {
            let detected = PanModule::detect(self.path.clone())?
                .ok_or(anyhow!("Could not detect package"))?;
            Ok(vec![ detected ])
        } else {
            Ok(modules)
        }
    }

    fn extract_master(&self) -> anyhow::Result<PanModule> {
        let maybe_master = self.conf.extract_master_mod()?;
        if let Some(master) = maybe_master {
            Ok(master)
        } else {
            let detected = PanModule::detect(self.path.clone())?
                .ok_or(anyhow!("Could not detect package"))?;
            Ok(detected)
        }
    }
}