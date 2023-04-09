use std::path::{Path, PathBuf};

use anyhow::{anyhow, Context};
use chrono::Utc;
use regex::Regex;

use crate::args::RelArgs;
use crate::git::GitRepo;
use crate::project::config::{PanProjectConfig, VcsConfig};
use crate::project::module::PanModule;
use crate::system::FileSystem;

const UNRELEASED_LINE: &str = "\n## [Unreleased]";

pub struct PanProject<F> {
    path: PathBuf,
    conf: PanProjectConfig<F>,
    repo: GitRepo,
}

impl <F: FileSystem + 'static> PanProject<F> {
    pub fn load(path: &Path) -> anyhow::Result<Self> {
        let project_root = GitRepo::find_git_root::<F>(path)
            .context("Error extracting project path from repo")?;
        let conf = PanProjectConfig::load(project_root)?;

        let git_conf = match conf.vcs() {
            VcsConfig::Git(git_conf) => git_conf,
        };
        let repo = GitRepo::open::<F>(git_conf.clone(), path)?;

        Ok(Self {
            path: path.to_path_buf(),
            conf,
            repo,
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
        if F::is_a_file(&changelog_path) {
            let mut changelog_content = F::read_string(&changelog_path)?;
            if !changelog_content.contains("\n## ") {
                changelog_content.push_str(UNRELEASED_LINE);
            } else if !changelog_content.contains(UNRELEASED_LINE) {
                changelog_content = Regex::new("(?=\n## )")
                    .expect("Invalid regex")
                    .replace(&changelog_content, UNRELEASED_LINE)
                    .to_string();
            }

            let updated_changelog = changelog_content.replace(UNRELEASED_LINE, &format!("{UNRELEASED_LINE}\n\n## [{version}] {}", Utc::now().format("%Y-%m-%d")));
            F::write_string(&changelog_path, &updated_changelog)?;
        }
        Ok(())
    }

    fn extract_modules(&self) -> anyhow::Result<Vec<PanModule<F>>> {
        let modules = self.conf.modules()?;
        if modules.is_empty() {
            let detected = PanModule::detect(self.path.clone())?
                .ok_or_else(|| anyhow!("Could not detect package"))?;
            Ok(vec![ detected ])
        } else {
            Ok(modules)
        }
    }

    fn extract_master(&self) -> anyhow::Result<PanModule<F>> {
        let maybe_master = self.conf.extract_master_mod()?;
        if let Some(master) = maybe_master {
            Ok(master)
        } else {
            let detected = PanModule::detect(self.path.clone())?
                .ok_or_else(|| anyhow!("Could not detect package"))?;
            Ok(detected)
        }
    }
}