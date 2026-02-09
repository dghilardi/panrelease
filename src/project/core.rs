use std::path::{Path, PathBuf};

use anyhow::{anyhow, Context};
use chrono::Utc;
use regex::Regex;

use crate::args::RelArgs;
use crate::changelog;
use crate::git::GitRepo;
use crate::project::config::{PanProjectConfig, VcsConfig};
use crate::project::module::PanModule;
use crate::project::strict;
use crate::system::FileSystem;

const UNRELEASED_LINE: &str = "\n## [Unreleased]";
const UNRELEASED_HEADING: &str = "## [Unreleased]";

/// Extract the text between `## [Unreleased]` and the next `## [` heading.
/// Returns the extracted text and the 1-indexed line number where it starts.
fn extract_unreleased_section(content: &str) -> (String, usize) {
    let lines: Vec<&str> = content.lines().collect();

    let unreleased_idx = lines.iter().position(|l| l.trim() == UNRELEASED_HEADING);
    let start = match unreleased_idx {
        Some(idx) => idx + 1,
        None => return (String::new(), 1),
    };

    let end = lines[start..]
        .iter()
        .position(|l| {
            let trimmed = l.trim();
            trimmed.starts_with("## [") && trimmed != UNRELEASED_HEADING
        })
        .map(|pos| start + pos)
        .unwrap_or(lines.len());

    let section_text = lines[start..end].join("\n");
    // Line numbers are 1-indexed
    (section_text, start + 1)
}

/// Replace the content between `## [Unreleased]` and the next version heading
/// with the new populated content.
fn replace_unreleased_content(content: &str, new_content: &str) -> String {
    let lines: Vec<&str> = content.lines().collect();

    let unreleased_idx = match lines.iter().position(|l| l.trim() == UNRELEASED_HEADING) {
        Some(idx) => idx,
        None => return content.to_string(),
    };

    let start = unreleased_idx + 1;

    let end = lines[start..]
        .iter()
        .position(|l| {
            let trimmed = l.trim();
            trimmed.starts_with("## [") && trimmed != UNRELEASED_HEADING
        })
        .map(|pos| start + pos)
        .unwrap_or(lines.len());

    let mut result = lines[..=unreleased_idx].join("\n");
    result.push_str(new_content);
    if end < lines.len() {
        result.push('\n');
        result.push_str(&lines[end..].join("\n"));
    }

    result
}

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

        let current_version = self.current_version()?;

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

            // Populate changelog from commits if enabled
            let changelog_conf = self.conf.changelog();
            if changelog_conf.from_commits {
                if let Ok(Some(prev_tag)) = self.repo.latest_version_tag("HEAD") {
                    if let Ok(commits) = self.repo.commits_since_tag(&prev_tag) {
                        let blame_lines = self.repo.blame_file(&changelog_path).unwrap_or_default();

                        let (unreleased_text, start_line) =
                            extract_unreleased_section(&changelog_content);

                        let populated = changelog::populate_changelog(
                            &unreleased_text,
                            start_line,
                            &commits,
                            &blame_lines,
                            changelog_conf,
                        );

                        if !populated.is_empty() {
                            changelog_content =
                                replace_unreleased_content(&changelog_content, &populated);
                        }
                    }
                }
            }

            let updated_changelog = changelog_content.replace(UNRELEASED_LINE, &format!("{UNRELEASED_LINE}\n\n## [{version}] {}", Utc::now().format("%Y-%m-%d")));
            F::write_string(&changelog_path, &updated_changelog)?;
        }
        Ok(())
    }

    pub fn current_version(&self) -> anyhow::Result<semver::Version> {
        self.extract_master()?.extract_version()
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