use std::path::{Path, PathBuf};

use anyhow::{bail, Context};

use crate::args::{SkillAction, SkillArgs};
use crate::git::GitRepo;
use crate::system::FileSystem;

/// The agent skill bundled with this panrelease version.
pub const SKILL_MD: &str = include_str!("../../skills/panrelease/SKILL.md");

/// Skill directories read by the main coding agents, relative to a project root or home.
/// `.agents/skills` is shared by Codex, Gemini CLI, Cursor and others; Claude Code uses `.claude/skills`.
const SKILL_DIRS: [&str; 2] = [".agents/skills", ".claude/skills"];

/// Agent instruction files updated with the versioning block, in order of preference.
const INSTRUCTION_FILES: [&str; 2] = ["AGENTS.md", "CLAUDE.md"];

const BLOCK_START: &str = "<!-- panrelease:start -->";
const BLOCK_END: &str = "<!-- panrelease:end -->";
const INSTRUCTIONS_BLOCK: &str = "<!-- panrelease:start -->
## Versioning

This repository is released with panrelease (`.panproject.toml`).
- Do not edit version numbers or create release tags by hand: use `panrelease release <patch|minor|major|post>`.
- Before releasing, commit all changes and make sure `## [Unreleased]` in CHANGELOG.md describes them.
- Never push release commits or tags without explicit confirmation.
- Follow the `panrelease` skill for the full workflow.
<!-- panrelease:end -->
";

#[derive(Debug, PartialEq, Eq)]
pub enum WriteOutcome {
    Created,
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
            report(&install_skill::<F>(&base)?);
            if install_args.agents_md {
                report(&add_agent_instructions::<F>(&base)?);
            }
        }
    }
    Ok(())
}

pub fn report(results: &[(PathBuf, WriteOutcome)]) {
    for (file, outcome) in results {
        let label = match outcome {
            WriteOutcome::Created => "Created",
            WriteOutcome::Updated => "Updated",
            WriteOutcome::UpToDate => "Up to date",
        };
        println!("{label}: {}", file.display());
    }
}

/// Write the bundled SKILL.md into every agent skill directory under `base`.
pub fn install_skill<F: FileSystem>(base: &Path) -> anyhow::Result<Vec<(PathBuf, WriteOutcome)>> {
    SKILL_DIRS
        .iter()
        .map(|dir| {
            let skill_dir = base.join(dir).join("panrelease");
            F::create_dir_all(&skill_dir)
                .with_context(|| format!("Failed to create {}", skill_dir.display()))?;
            let file = skill_dir.join("SKILL.md");
            let outcome = write_if_changed::<F>(&file, |_| Ok(SKILL_MD.to_string()))?;
            Ok((file, outcome))
        })
        .collect()
}

/// Insert or refresh the versioning block in the existing agent instruction files under `root`,
/// creating AGENTS.md when none exists.
pub fn add_agent_instructions<F: FileSystem>(root: &Path) -> anyhow::Result<Vec<(PathBuf, WriteOutcome)>> {
    let mut targets: Vec<PathBuf> = INSTRUCTION_FILES
        .iter()
        .map(|name| root.join(name))
        .filter(|file| F::is_a_file(file))
        .collect();
    if targets.is_empty() {
        targets.push(root.join(INSTRUCTION_FILES[0]));
    }
    targets
        .into_iter()
        .map(|file| {
            let outcome = write_if_changed::<F>(&file, |current| {
                upsert_block(current.unwrap_or_default())
                    .with_context(|| format!("Cannot update {}", file.display()))
            })?;
            Ok((file, outcome))
        })
        .collect()
}

fn write_if_changed<F: FileSystem>(
    file: &Path,
    render: impl FnOnce(Option<&str>) -> anyhow::Result<String>,
) -> anyhow::Result<WriteOutcome> {
    let current = if F::is_a_file(file) { Some(F::read_string(file)?) } else { None };
    let content = render(current.as_deref())?;
    let outcome = match current {
        None => WriteOutcome::Created,
        Some(current) if current == content => return Ok(WriteOutcome::UpToDate),
        Some(_) => WriteOutcome::Updated,
    };
    F::write_string(file, &content).with_context(|| format!("Failed to write {}", file.display()))?;
    Ok(outcome)
}

fn upsert_block(content: &str) -> anyhow::Result<String> {
    match (content.find(BLOCK_START), content.find(BLOCK_END)) {
        (Some(start), Some(end)) if start < end => {
            let mut end = end + BLOCK_END.len();
            if content[end..].starts_with('\n') {
                end += 1;
            }
            Ok(format!("{}{INSTRUCTIONS_BLOCK}{}", &content[..start], &content[end..]))
        }
        (None, None) if content.trim().is_empty() => Ok(INSTRUCTIONS_BLOCK.to_string()),
        (None, None) => {
            let separator = if content.ends_with("\n\n") {
                ""
            } else if content.ends_with('\n') {
                "\n"
            } else {
                "\n\n"
            };
            Ok(format!("{content}{separator}{INSTRUCTIONS_BLOCK}"))
        }
        _ => bail!("unbalanced panrelease markers ({BLOCK_START} ... {BLOCK_END}); fix them manually"),
    }
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
        let result = install_skill::<NativeSystem>(tmp.path()).unwrap();

        assert_eq!(result.len(), SKILL_DIRS.len());
        for dir in SKILL_DIRS {
            let file = tmp.path().join(dir).join("panrelease/SKILL.md");
            assert_eq!(std::fs::read_to_string(&file).unwrap(), SKILL_MD);
        }
        assert!(result.iter().all(|(_, o)| *o == WriteOutcome::Created));
    }

    #[test]
    fn reinstall_is_idempotent_and_updates_stale_copies() {
        let tmp = tempfile::tempdir().unwrap();
        install_skill::<NativeSystem>(tmp.path()).unwrap();
        let stale = tmp.path().join(".claude/skills/panrelease/SKILL.md");
        std::fs::write(&stale, "old").unwrap();

        let result = install_skill::<NativeSystem>(tmp.path()).unwrap();

        assert_eq!(result[0].1, WriteOutcome::UpToDate);
        assert_eq!(result[1].1, WriteOutcome::Updated);
        assert_eq!(std::fs::read_to_string(&stale).unwrap(), SKILL_MD);
    }

    #[test]
    fn instructions_create_agents_md_when_no_file_exists() {
        let tmp = tempfile::tempdir().unwrap();
        let result = add_agent_instructions::<NativeSystem>(tmp.path()).unwrap();

        assert_eq!(result, vec![(tmp.path().join("AGENTS.md"), WriteOutcome::Created)]);
        assert_eq!(std::fs::read_to_string(tmp.path().join("AGENTS.md")).unwrap(), INSTRUCTIONS_BLOCK);
        assert!(!tmp.path().join("CLAUDE.md").exists());
    }

    #[test]
    fn instructions_update_only_existing_files() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join("CLAUDE.md"), "# Project\n").unwrap();

        let result = add_agent_instructions::<NativeSystem>(tmp.path()).unwrap();

        assert_eq!(result, vec![(tmp.path().join("CLAUDE.md"), WriteOutcome::Updated)]);
        assert_eq!(
            std::fs::read_to_string(tmp.path().join("CLAUDE.md")).unwrap(),
            format!("# Project\n\n{INSTRUCTIONS_BLOCK}")
        );
        assert!(!tmp.path().join("AGENTS.md").exists());
    }

    #[test]
    fn instructions_are_idempotent() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join("AGENTS.md"), "# Agents\n").unwrap();
        add_agent_instructions::<NativeSystem>(tmp.path()).unwrap();

        let result = add_agent_instructions::<NativeSystem>(tmp.path()).unwrap();

        assert_eq!(result[0].1, WriteOutcome::UpToDate);
    }

    #[test]
    fn upsert_replaces_existing_block_and_keeps_surrounding_text() {
        let content = format!("# Agents\n\n{BLOCK_START}\nold\n{BLOCK_END}\n\n## Other\n");
        assert_eq!(
            upsert_block(&content).unwrap(),
            format!("# Agents\n\n{INSTRUCTIONS_BLOCK}\n## Other\n")
        );
    }

    #[test]
    fn upsert_rejects_unbalanced_markers() {
        assert!(upsert_block(&format!("# Agents\n{BLOCK_START}\n")).is_err());
    }
}
