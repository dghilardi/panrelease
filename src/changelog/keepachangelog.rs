use std::collections::{BTreeMap, HashSet};

use crate::project::config::ChangelogConfig;

use super::commit_format::{parse_commit, ChangelogSection, ParsedCommit};
use super::{BlameLine, CommitInfo};

/// An entry already present in the unreleased section of the changelog.
#[derive(Debug, Clone)]
struct ExistingEntry {
    text: String,
    line_number: usize,
}

/// A new entry generated from a parsed commit.
#[derive(Debug, Clone)]
struct NewEntry {
    text: String,
    timestamp: i64,
}

/// Merged entry for final rendering, with a sort key.
#[derive(Debug, Clone)]
struct MergedEntry {
    text: String,
    timestamp: i64,
}

/// Parse the unreleased section content into a map of sections to entries.
/// Each entry tracks its line number (1-indexed, relative to the full file)
/// so we can correlate with git blame.
fn parse_unreleased_sections(
    unreleased_text: &str,
    start_line: usize,
) -> BTreeMap<ChangelogSection, Vec<ExistingEntry>> {
    let mut sections: BTreeMap<ChangelogSection, Vec<ExistingEntry>> = BTreeMap::new();
    let mut current_section: Option<ChangelogSection> = None;

    for (i, line) in unreleased_text.lines().enumerate() {
        let trimmed = line.trim();

        if let Some(header) = trimmed.strip_prefix("### ") {
            current_section = ChangelogSection::from_header(header);
        } else if trimmed.starts_with("- ") || trimmed.starts_with("* ") {
            if let Some(ref section) = current_section {
                sections.entry(section.clone()).or_default().push(ExistingEntry {
                    text: trimmed.to_string(),
                    line_number: start_line + i,
                });
            }
        }
    }

    sections
}

/// Build a set of commit hashes that already have entries in the changelog,
/// based on git blame data.
fn build_blame_commit_set(
    existing_sections: &BTreeMap<ChangelogSection, Vec<ExistingEntry>>,
    blame_lines: &[BlameLine],
) -> HashSet<String> {
    let mut commit_set = HashSet::new();

    for entries in existing_sections.values() {
        for entry in entries {
            // line_number is 1-indexed, blame_lines is 0-indexed
            if entry.line_number > 0 {
                if let Some(blame) = blame_lines.get(entry.line_number - 1) {
                    commit_set.insert(blame.commit_hash.clone());
                }
            }
        }
    }

    commit_set
}

/// Format a parsed commit into a changelog entry line.
fn format_entry(parsed: &ParsedCommit, config: &ChangelogConfig) -> String {
    let mut entry = String::from("- ");

    if parsed.breaking {
        entry.push_str("[BREAKING] ");
    }

    if config.include_scope {
        if let Some(ref scope) = parsed.scope {
            entry.push_str(&format!("({scope}) "));
        }
    }

    entry.push_str(&parsed.description);
    entry
}

/// Look up the timestamp for an existing entry via blame.
/// Returns the commit timestamp from the blame data if available.
fn entry_timestamp(entry: &ExistingEntry, blame_lines: &[BlameLine], commits: &[CommitInfo]) -> i64 {
    if entry.line_number == 0 {
        return 0;
    }
    if let Some(blame) = blame_lines.get(entry.line_number - 1) {
        for commit in commits {
            if commit.hash == blame.commit_hash {
                return commit.timestamp;
            }
        }
    }
    0
}

/// Merge existing and new entries for a section, ordered by timestamp (newest first).
fn merge_entries(
    existing: &[ExistingEntry],
    new: &[NewEntry],
    blame_lines: &[BlameLine],
    commits: &[CommitInfo],
) -> Vec<String> {
    let mut merged: Vec<MergedEntry> = Vec::new();

    for entry in existing {
        let ts = entry_timestamp(entry, blame_lines, commits);
        merged.push(MergedEntry {
            text: entry.text.clone(),
            timestamp: ts,
        });
    }

    for entry in new {
        merged.push(MergedEntry {
            text: entry.text.clone(),
            timestamp: entry.timestamp,
        });
    }

    // Sort by timestamp descending (newest first), stable sort preserves
    // relative order for entries with the same timestamp
    merged.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));

    merged.into_iter().map(|e| e.text).collect()
}

/// Render sections into keepachangelog markdown text.
/// Only includes sections that have at least one entry.
/// Section order follows keepachangelog convention.
fn render_sections(sections: &BTreeMap<ChangelogSection, Vec<String>>) -> String {
    let section_order = [
        ChangelogSection::Added,
        ChangelogSection::Changed,
        ChangelogSection::Deprecated,
        ChangelogSection::Removed,
        ChangelogSection::Fixed,
        ChangelogSection::Security,
        ChangelogSection::Other,
    ];

    let mut output = String::new();

    for section in &section_order {
        if let Some(entries) = sections.get(section) {
            if entries.is_empty() {
                continue;
            }
            output.push_str(&format!("\n### {section}\n"));
            for entry in entries {
                output.push_str(&format!("{entry}\n"));
            }
        }
    }

    output
}

/// Main function: populate changelog from commits, merging with existing content.
pub fn populate(
    unreleased_text: &str,
    unreleased_start_line: usize,
    commits: &[CommitInfo],
    blame_lines: &[BlameLine],
    config: &ChangelogConfig,
    format: &crate::project::config::CommitFormatKind,
) -> String {
    // Parse existing entries
    let existing_sections = parse_unreleased_sections(unreleased_text, unreleased_start_line);

    // Build set of commits already represented
    let blame_set = build_blame_commit_set(&existing_sections, blame_lines);

    // Parse new commits and group by section
    let mut new_entries: BTreeMap<ChangelogSection, Vec<NewEntry>> = BTreeMap::new();

    for commit in commits {
        if blame_set.contains(&commit.hash) {
            continue;
        }

        if let Some(parsed) = parse_commit(&commit.message, format) {
            let entry_text = format_entry(&parsed, config);
            new_entries.entry(parsed.section).or_default().push(NewEntry {
                text: entry_text,
                timestamp: commit.timestamp,
            });
        } else if config.include_unmatched {
            let desc = commit.message.lines().next().unwrap_or("").trim();
            if !desc.is_empty() {
                let entry_text = format!("- {}", super::commit_format::capitalize_first(desc));
                new_entries.entry(ChangelogSection::Other).or_default().push(NewEntry {
                    text: entry_text,
                    timestamp: commit.timestamp,
                });
            }
        }
    }

    // Merge existing and new entries per section
    let mut merged_sections: BTreeMap<ChangelogSection, Vec<String>> = BTreeMap::new();

    // Collect all section keys
    let all_sections: HashSet<ChangelogSection> = existing_sections
        .keys()
        .chain(new_entries.keys())
        .cloned()
        .collect();

    for section in all_sections {
        let existing = existing_sections.get(&section).cloned().unwrap_or_default();
        let new = new_entries.get(&section).cloned().unwrap_or_default();
        let entries = merge_entries(&existing, &new, blame_lines, commits);
        if !entries.is_empty() {
            merged_sections.insert(section, entries);
        }
    }

    render_sections(&merged_sections)
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::project::config::{ChangelogConfig, CommitFormatKind};

    fn default_config() -> ChangelogConfig {
        ChangelogConfig {
            from_commits: true,
            commit_format: CommitFormatKind::Conventional,
            include_scope: true,
            include_unmatched: false,
        }
    }

    // --- parse_unreleased_sections ---

    #[test]
    fn parse_empty_unreleased() {
        let sections = parse_unreleased_sections("", 1);
        assert!(sections.is_empty());
    }

    #[test]
    fn parse_single_section() {
        let text = "### Added\n- First feature\n- Second feature\n";
        let sections = parse_unreleased_sections(text, 10);
        assert_eq!(1, sections.len());
        let added = sections.get(&ChangelogSection::Added).unwrap();
        assert_eq!(2, added.len());
        assert_eq!("- First feature", added[0].text);
        assert_eq!(11, added[0].line_number); // line 10 + 1 (header is line 10, first entry is line 11)
        assert_eq!("- Second feature", added[1].text);
    }

    #[test]
    fn parse_multiple_sections() {
        let text = "### Added\n- New feature\n\n### Fixed\n- Bug fix\n";
        let sections = parse_unreleased_sections(text, 1);
        assert_eq!(2, sections.len());
        assert!(sections.contains_key(&ChangelogSection::Added));
        assert!(sections.contains_key(&ChangelogSection::Fixed));
    }

    #[test]
    fn parse_entries_outside_section_ignored() {
        let text = "- orphan entry\n### Added\n- real entry\n";
        let sections = parse_unreleased_sections(text, 1);
        assert_eq!(1, sections.len());
        let added = sections.get(&ChangelogSection::Added).unwrap();
        assert_eq!(1, added.len());
    }

    // --- format_entry ---

    #[test]
    fn format_entry_simple() {
        let parsed = ParsedCommit {
            section: ChangelogSection::Added,
            scope: None,
            description: "Add feature".into(),
            breaking: false,
        };
        assert_eq!("- Add feature", format_entry(&parsed, &default_config()));
    }

    #[test]
    fn format_entry_with_scope() {
        let parsed = ParsedCommit {
            section: ChangelogSection::Added,
            scope: Some("auth".into()),
            description: "Add OAuth".into(),
            breaking: false,
        };
        assert_eq!("- (auth) Add OAuth", format_entry(&parsed, &default_config()));
    }

    #[test]
    fn format_entry_scope_excluded() {
        let parsed = ParsedCommit {
            section: ChangelogSection::Added,
            scope: Some("auth".into()),
            description: "Add OAuth".into(),
            breaking: false,
        };
        let config = ChangelogConfig {
            include_scope: false,
            ..default_config()
        };
        assert_eq!("- Add OAuth", format_entry(&parsed, &config));
    }

    #[test]
    fn format_entry_breaking() {
        let parsed = ParsedCommit {
            section: ChangelogSection::Changed,
            scope: None,
            description: "Rewrite API".into(),
            breaking: true,
        };
        assert_eq!("- [BREAKING] Rewrite API", format_entry(&parsed, &default_config()));
    }

    #[test]
    fn format_entry_breaking_with_scope() {
        let parsed = ParsedCommit {
            section: ChangelogSection::Changed,
            scope: Some("api".into()),
            description: "Rewrite API".into(),
            breaking: true,
        };
        assert_eq!("- [BREAKING] (api) Rewrite API", format_entry(&parsed, &default_config()));
    }

    // --- render_sections ---

    #[test]
    fn render_empty() {
        let sections = BTreeMap::new();
        assert_eq!("", render_sections(&sections));
    }

    #[test]
    fn render_single_section() {
        let mut sections = BTreeMap::new();
        sections.insert(ChangelogSection::Added, vec!["- Feature".into()]);
        let rendered = render_sections(&sections);
        assert_eq!("\n### Added\n- Feature\n", rendered);
    }

    #[test]
    fn render_section_order() {
        let mut sections = BTreeMap::new();
        sections.insert(ChangelogSection::Fixed, vec!["- Bug fix".into()]);
        sections.insert(ChangelogSection::Added, vec!["- Feature".into()]);
        let rendered = render_sections(&sections);
        // Added should come before Fixed
        let added_pos = rendered.find("### Added").unwrap();
        let fixed_pos = rendered.find("### Fixed").unwrap();
        assert!(added_pos < fixed_pos);
    }

    #[test]
    fn render_skips_empty_sections() {
        let mut sections = BTreeMap::new();
        sections.insert(ChangelogSection::Added, vec![]);
        sections.insert(ChangelogSection::Fixed, vec!["- Bug fix".into()]);
        let rendered = render_sections(&sections);
        assert!(!rendered.contains("### Added"));
        assert!(rendered.contains("### Fixed"));
    }

    // --- populate (end-to-end) ---

    #[test]
    fn populate_with_new_commits() {
        let commits = vec![
            CommitInfo { hash: "abc123".into(), message: "feat: add login".into(), timestamp: 200 },
            CommitInfo { hash: "def456".into(), message: "fix: resolve crash".into(), timestamp: 100 },
        ];
        let config = default_config();
        let format = CommitFormatKind::Conventional;

        let result = populate("", 1, &commits, &[], &config, &format);
        assert!(result.contains("### Added"));
        assert!(result.contains("- Add login"));
        assert!(result.contains("### Fixed"));
        assert!(result.contains("- Resolve crash"));
    }

    #[test]
    fn populate_preserves_existing() {
        let existing = "### Added\n- Existing feature\n";
        let commits = vec![
            CommitInfo { hash: "abc123".into(), message: "fix: fix bug".into(), timestamp: 100 },
        ];
        let config = default_config();
        let format = CommitFormatKind::Conventional;

        let result = populate(existing, 1, &commits, &[], &config, &format);
        assert!(result.contains("- Existing feature"));
        assert!(result.contains("- Fix bug"));
    }

    #[test]
    fn populate_deduplicates_via_blame() {
        let existing = "### Added\n- Add login\n";
        let blame_lines = vec![
            BlameLine { commit_hash: "header".into(), line_content: "### Added".into() },
            BlameLine { commit_hash: "abc123".into(), line_content: "- Add login".into() },
        ];
        let commits = vec![
            CommitInfo { hash: "abc123".into(), message: "feat: add login".into(), timestamp: 100 },
        ];
        let config = default_config();
        let format = CommitFormatKind::Conventional;

        let result = populate(existing, 1, &commits, &blame_lines, &config, &format);
        // Should only have one entry for "Add login", not duplicated
        assert_eq!(1, result.matches("Add login").count());
    }

    #[test]
    fn populate_include_unmatched() {
        let commits = vec![
            CommitInfo { hash: "abc".into(), message: "random commit message".into(), timestamp: 100 },
        ];
        let config = ChangelogConfig {
            include_unmatched: true,
            ..default_config()
        };
        let format = CommitFormatKind::Conventional;

        let result = populate("", 1, &commits, &[], &config, &format);
        assert!(result.contains("### Other"));
        assert!(result.contains("- Random commit message"));
    }

    #[test]
    fn populate_exclude_unmatched() {
        let commits = vec![
            CommitInfo { hash: "abc".into(), message: "random commit message".into(), timestamp: 100 },
        ];
        let config = default_config(); // include_unmatched = false
        let format = CommitFormatKind::Conventional;

        let result = populate("", 1, &commits, &[], &config, &format);
        assert!(result.is_empty());
    }

    #[test]
    fn populate_empty_commits() {
        let config = default_config();
        let format = CommitFormatKind::Conventional;
        let result = populate("", 1, &[], &[], &config, &format);
        assert!(result.is_empty());
    }

    #[test]
    fn populate_all_ignored_commits() {
        let commits = vec![
            CommitInfo { hash: "a".into(), message: "docs: update readme".into(), timestamp: 100 },
            CommitInfo { hash: "b".into(), message: "chore: clean up".into(), timestamp: 90 },
        ];
        let config = default_config();
        let format = CommitFormatKind::Conventional;
        let result = populate("", 1, &commits, &[], &config, &format);
        assert!(result.is_empty());
    }

    #[test]
    fn populate_ordering_newest_first() {
        let commits = vec![
            CommitInfo { hash: "a".into(), message: "feat: newer feature".into(), timestamp: 200 },
            CommitInfo { hash: "b".into(), message: "feat: older feature".into(), timestamp: 100 },
        ];
        let config = default_config();
        let format = CommitFormatKind::Conventional;

        let result = populate("", 1, &commits, &[], &config, &format);
        let newer_pos = result.find("Newer feature").unwrap();
        let older_pos = result.find("Older feature").unwrap();
        assert!(newer_pos < older_pos);
    }
}
