use std::path::{Path, PathBuf};

use anyhow::Context;

use crate::args::{SkillAction, SkillArgs};
use crate::git::GitRepo;
use crate::system::FileSystem;

/// The agent skill bundled with this panrelease version.
pub const SKILL_MD: &str = include_str!("../../skills/panrelease/SKILL.md");

/// Skill directories read by the main coding agents, relative to a project root or home.
/// `.agents/skills` is shared by Codex, Gemini CLI, Cursor and others; Claude Code uses `.claude/skills`.
const SKILL_DIRS: [&str; 2] = [".agents/skills", ".claude/skills"];

#[derive(Debug, PartialEq, Eq)]
enum InstallOutcome {
    Installed,
    Updated,
    UpToDate,
}

pub fn run<F: FileSystem + 'static>(path: Option<PathBuf>, args: SkillArgs) -> anyhow::Result<()> {
    match args.action {
        SkillAction::Print => print!("{SKILL_MD}"),
        SkillAction::Install(install_args) => {
            let base = if install_args.global {
                F::home_dir()?
            } else {
                let cwd = match path {
                    Some(path) if path.is_absolute() => path,
                    Some(path) => F::current_dir()?.join(path),
                    None => F::current_dir()?,
                };
                GitRepo::find_git_root::<F>(&cwd)
                    .context("Could not find git repository root (use --global to install in the home directory)")?
                    .to_path_buf()
            };
            for (file, outcome) in install_at::<F>(&base)? {
                let label = match outcome {
                    InstallOutcome::Installed => "Installed",
                    InstallOutcome::Updated => "Updated",
                    InstallOutcome::UpToDate => "Up to date",
                };
                println!("{label}: {}", file.display());
            }
        }
    }
    Ok(())
}

fn install_at<F: FileSystem>(base: &Path) -> anyhow::Result<Vec<(PathBuf, InstallOutcome)>> {
    SKILL_DIRS
        .iter()
        .map(|dir| {
            let skill_dir = base.join(dir).join("panrelease");
            let file = skill_dir.join("SKILL.md");
            let outcome = if !F::is_a_file(&file) {
                InstallOutcome::Installed
            } else if F::read_string(&file)? == SKILL_MD {
                InstallOutcome::UpToDate
            } else {
                InstallOutcome::Updated
            };
            if outcome != InstallOutcome::UpToDate {
                F::create_dir_all(&skill_dir)
                    .with_context(|| format!("Failed to create {}", skill_dir.display()))?;
                F::write_string(&file, SKILL_MD)
                    .with_context(|| format!("Failed to write {}", file.display()))?;
            }
            Ok((file, outcome))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::system::NativeSystem;

    #[test]
    fn bundled_skill_has_frontmatter_name() {
        assert!(SKILL_MD.starts_with("---\nname: panrelease\n"));
    }

    #[test]
    fn install_writes_all_skill_dirs() {
        let tmp = tempfile::tempdir().unwrap();
        let result = install_at::<NativeSystem>(tmp.path()).unwrap();

        assert_eq!(result.len(), SKILL_DIRS.len());
        for dir in SKILL_DIRS {
            let file = tmp.path().join(dir).join("panrelease/SKILL.md");
            assert_eq!(std::fs::read_to_string(&file).unwrap(), SKILL_MD);
        }
        assert!(result.iter().all(|(_, o)| *o == InstallOutcome::Installed));
    }

    #[test]
    fn reinstall_is_idempotent_and_updates_stale_copies() {
        let tmp = tempfile::tempdir().unwrap();
        install_at::<NativeSystem>(tmp.path()).unwrap();
        let stale = tmp.path().join(".claude/skills/panrelease/SKILL.md");
        std::fs::write(&stale, "old").unwrap();

        let result = install_at::<NativeSystem>(tmp.path()).unwrap();

        assert_eq!(result[0].1, InstallOutcome::UpToDate);
        assert_eq!(result[1].1, InstallOutcome::Updated);
        assert_eq!(std::fs::read_to_string(&stale).unwrap(), SKILL_MD);
    }
}
