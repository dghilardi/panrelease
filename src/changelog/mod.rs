pub mod commit_format;
pub mod keepachangelog;

use crate::project::config::{ChangelogConfig, CommitFormatKind};

use self::commit_format::detect_format;

#[derive(Debug, Clone)]
pub struct CommitInfo {
    pub hash: String,
    pub message: String,
    pub timestamp: i64,
}

#[derive(Debug, Clone)]
pub struct BlameLine {
    pub commit_hash: String,
    #[allow(dead_code)]
    pub line_content: String,
}

/// Main entry point: populate the unreleased section of a changelog from commit messages.
///
/// `unreleased_text` is the text currently under `## [Unreleased]` (between the heading
/// and the next `## [` heading).
/// `unreleased_start_line` is the 1-indexed line number in the file where the unreleased
/// section content starts (used for blame correlation).
/// `commits` are the commits since the last tag.
/// `blame_lines` are the blame results for the changelog file.
/// `config` is the changelog configuration from `.panproject.toml`.
///
/// Returns the new content to place under `## [Unreleased]`.
pub fn populate_changelog(
    unreleased_text: &str,
    unreleased_start_line: usize,
    commits: &[CommitInfo],
    blame_lines: &[BlameLine],
    config: &ChangelogConfig,
) -> String {
    let format = match &config.commit_format {
        CommitFormatKind::Auto => detect_format(commits),
        other => other.clone(),
    };

    keepachangelog::populate(
        unreleased_text,
        unreleased_start_line,
        commits,
        blame_lines,
        config,
        &format,
    )
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn populate_changelog_auto_detects_conventional() {
        let commits = vec![
            CommitInfo { hash: "a".into(), message: "feat: add feature".into(), timestamp: 200 },
            CommitInfo { hash: "b".into(), message: "fix: fix bug".into(), timestamp: 100 },
        ];
        let config = ChangelogConfig {
            from_commits: true,
            commit_format: CommitFormatKind::Auto,
            include_scope: true,
            include_unmatched: false,
        };

        let result = populate_changelog("", 1, &commits, &[], &config);
        assert!(result.contains("### Added"));
        assert!(result.contains("- Add feature"));
        assert!(result.contains("### Fixed"));
        assert!(result.contains("- Fix bug"));
    }

    #[test]
    fn populate_changelog_auto_detects_gitmoji() {
        let commits = vec![
            CommitInfo { hash: "a".into(), message: ":sparkles: add feature".into(), timestamp: 200 },
            CommitInfo { hash: "b".into(), message: ":bug: fix bug".into(), timestamp: 100 },
        ];
        let config = ChangelogConfig {
            from_commits: true,
            commit_format: CommitFormatKind::Auto,
            include_scope: true,
            include_unmatched: false,
        };

        let result = populate_changelog("", 1, &commits, &[], &config);
        assert!(result.contains("### Added"));
        assert!(result.contains("- Add feature"));
    }

    #[test]
    fn populate_changelog_empty_commits() {
        let config = ChangelogConfig {
            from_commits: true,
            commit_format: CommitFormatKind::Conventional,
            include_scope: true,
            include_unmatched: false,
        };

        let result = populate_changelog("", 1, &[], &[], &config);
        assert!(result.is_empty());
    }

    #[test]
    fn populate_changelog_with_explicit_format() {
        let commits = vec![
            CommitInfo { hash: "a".into(), message: ":sparkles: add feature".into(), timestamp: 100 },
        ];
        let config = ChangelogConfig {
            from_commits: true,
            commit_format: CommitFormatKind::Gitmoji,
            include_scope: true,
            include_unmatched: false,
        };

        let result = populate_changelog("", 1, &commits, &[], &config);
        assert!(result.contains("- Add feature"));
    }
}
