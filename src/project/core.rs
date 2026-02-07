use std::path::{Path, PathBuf};

use anyhow::{anyhow, Context};
use chrono::Utc;
use regex::Regex;

use crate::args::RelArgs;
use crate::git::GitRepo;
use crate::project::config::{PanProjectConfig, VcsConfig};
use crate::project::module::PanModule;
use crate::project::strict;
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
            path: project_root.to_path_buf(),
            conf,
            repo,
        })
    }

    pub fn release(&self, rel_args: RelArgs) -> anyhow::Result<()> {
        if !self.repo.is_staging_clean()? {
            return Err(anyhow!("Repository status is not clean"));
        }

        let current_version = self.extract_master()?.extract_version()?;

        let strict_conf = match self.conf.vcs() {
            VcsConfig::Git(git_conf) => git_conf.strict.as_ref(),
        };

        let slug: Option<String> = if let Some(strict) = strict_conf {
            let branch = self.repo.current_branch()?
                .ok_or_else(|| anyhow!("Strict mode: cannot release from detached HEAD"))?;

            let branch_kind = strict::classify_branch(&branch, &strict.mainline)?;

            let base_version = match &branch_kind {
                strict::BranchKind::Mainline => None,
                _ => {
                    let tag_template = match self.conf.vcs() {
                        VcsConfig::Git(gc) => &gc.tag_template,
                    };
                    let merge_base_commit = self.repo.merge_base(&strict.mainline)?;
                    let tag = self.repo.latest_version_tag(&merge_base_commit)?
                        .ok_or_else(|| anyhow!(
                            "Strict mode: no version tag found reachable from \
                             merge-base with '{}'", strict.mainline
                        ))?;
                    let ver = strict::version_from_tag(&tag, tag_template)
                        .ok_or_else(|| anyhow!(
                            "Strict mode: could not parse version from tag '{tag}'"
                        ))?;
                    Some(ver)
                }
            };

            let inferred_slug = match &branch_kind {
                strict::BranchKind::Feature { slug } => Some(slug.as_str()),
                strict::BranchKind::Hotfix { slug } => Some(slug.as_str()),
                strict::BranchKind::Mainline => None,
            };

            let new_version = rel_args.level_or_version
                .apply_with_slug(current_version.clone(), inferred_slug);

            strict::validate_release(
                &branch_kind,
                &new_version,
                base_version.as_ref(),
            )?;

            inferred_slug.map(String::from)
        } else {
            None
        };

        let new_version = rel_args.level_or_version
            .apply_with_slug(current_version, slug.as_deref());

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