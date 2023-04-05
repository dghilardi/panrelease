use std::fs;
use std::marker::PhantomData;
use std::path::{Path, PathBuf};

use anyhow::anyhow;
use chrono::Utc;

use crate::args::RelArgs;
use crate::git::GitRepo;
use crate::project::config::PanProjectConfig;
use crate::project::module::PanModule;
use crate::system::FileSystem;

pub struct PanProject<F> {
    path: PathBuf,
    conf: PanProjectConfig,
    repo: GitRepo,
    filesystem: PhantomData<F>,
}

impl <F: FileSystem> PanProject<F> {
    pub fn load(path: &Path) -> anyhow::Result<Self> {
        let repo = GitRepo::open::<F>(path)?;
        let project_root = repo.path().parent()
            .ok_or_else(|| anyhow!("Error extracting project path from repo"))?;
        let conf = PanProjectConfig::load(project_root)?;

        Ok(Self {
            path: path.to_path_buf(),
            conf,
            repo,
            filesystem: PhantomData
        })
    }

    pub fn release(&self, rel_args: RelArgs) -> anyhow::Result<()> {
        if !self.repo.is_staging_clean()? {
            return Err(anyhow!("Repository status is not clean"));
        }
        let new_version = rel_args.level_or_version.apply(self.extract_master()?.extract_version()?);
        for mut module in self.extract_modules()? {
            module.set_version(&new_version)?;
            module.persist()?;
            module.hook_after_rel()?;
        }

        self.update_changelog(&new_version)?;
        self.repo.update_and_commit(new_version)?;

        Ok(())
    }

    fn update_changelog(&self, version: &semver::Version) -> anyhow::Result<()> {
        let changelog_path = self.path.join("CHANGELOG.md");
        if changelog_path.is_file() {
            let changelog_content = fs::read_to_string(&changelog_path)?;
            let updated_changelog = changelog_content.replace("\n## [Unreleased]", &format!("\n## [Unreleased]\n\n## [{version}] {}", Utc::now().format("%Y-%m-%d")));
            fs::write(&changelog_path, updated_changelog)?;
        }
        Ok(())
    }

    fn extract_modules(&self) -> anyhow::Result<Vec<PanModule>> {
        let modules = self.conf.modules()?;
        if modules.is_empty() {
            let detected = PanModule::detect::<F>(self.path.clone())?
                .ok_or_else(|| anyhow!("Could not detect package"))?;
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
            let detected = PanModule::detect::<F>(self.path.clone())?
                .ok_or_else(|| anyhow!("Could not detect package"))?;
            Ok(detected)
        }
    }
}