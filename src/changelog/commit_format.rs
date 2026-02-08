use regex::Regex;

use crate::project::config::CommitFormatKind;

use super::CommitInfo;

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ChangelogSection {
    Added,
    Changed,
    Deprecated,
    Removed,
    Fixed,
    Security,
    Other,
}

impl std::fmt::Display for ChangelogSection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ChangelogSection::Added => write!(f, "Added"),
            ChangelogSection::Changed => write!(f, "Changed"),
            ChangelogSection::Deprecated => write!(f, "Deprecated"),
            ChangelogSection::Removed => write!(f, "Removed"),
            ChangelogSection::Fixed => write!(f, "Fixed"),
            ChangelogSection::Security => write!(f, "Security"),
            ChangelogSection::Other => write!(f, "Other"),
        }
    }
}

impl ChangelogSection {
    pub fn from_header(header: &str) -> Option<Self> {
        match header.trim().to_lowercase().as_str() {
            "added" | "add" => Some(Self::Added),
            "changed" | "modified" => Some(Self::Changed),
            "deprecated" => Some(Self::Deprecated),
            "removed" => Some(Self::Removed),
            "fixed" | "fix" => Some(Self::Fixed),
            "security" => Some(Self::Security),
            "other" => Some(Self::Other),
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ParsedCommit {
    pub section: ChangelogSection,
    pub scope: Option<String>,
    pub description: String,
    pub breaking: bool,
}

pub fn capitalize_first(s: &str) -> String {
    let trimmed = s.trim();
    if trimmed.is_empty() {
        return String::new();
    }
    let mut chars = trimmed.chars();
    match chars.next() {
        Some(c) => c.to_uppercase().to_string() + chars.as_str(),
        None => String::new(),
    }
}

pub fn parse_conventional(message: &str) -> Option<ParsedCommit> {
    let subject = message.lines().next().unwrap_or("").trim();
    let re = Regex::new(
        r"^(?P<type>\w+)(?:\((?P<scope>[^)]+)\))?(?P<bang>!)?\s*:\s*(?P<desc>.+)"
    ).unwrap();

    let caps = re.captures(subject)?;
    let commit_type = caps.name("type")?.as_str().to_lowercase();
    let scope = caps.name("scope").map(|m| m.as_str().to_string());
    let bang = caps.name("bang").is_some();
    let desc = caps.name("desc")?.as_str();

    let section = match commit_type.as_str() {
        "feat" => ChangelogSection::Added,
        "fix" => ChangelogSection::Fixed,
        "refactor" | "perf" => ChangelogSection::Changed,
        "deprecate" => ChangelogSection::Deprecated,
        "security" => ChangelogSection::Security,
        "docs" | "chore" | "ci" | "build" | "style" | "test" => return None,
        _ => return None,
    };

    let body_has_breaking = message.lines().skip(1).any(|line| {
        let trimmed = line.trim();
        trimmed.starts_with("BREAKING CHANGE:") || trimmed.starts_with("BREAKING-CHANGE:")
    });

    let breaking = bang || body_has_breaking;

    let (final_section, description) = if breaking {
        (ChangelogSection::Changed, capitalize_first(desc))
    } else {
        (section, capitalize_first(desc))
    };

    Some(ParsedCommit {
        section: final_section,
        scope,
        description,
        breaking,
    })
}

pub fn parse_gitmoji(message: &str) -> Option<ParsedCommit> {
    let subject = message.lines().next().unwrap_or("").trim();

    // Match shortcode format :emoji: or common unicode emoji
    let re = Regex::new(r"^:(?P<emoji>[a-z_0-9]+):\s*(?P<desc>.+)").unwrap();

    if let Some(caps) = re.captures(subject) {
        let emoji = caps.name("emoji")?.as_str();
        let desc = caps.name("desc")?.as_str();
        return gitmoji_to_parsed(emoji, desc);
    }

    // Match unicode emoji at start of line
    let unicode_map = [
        ("\u{2728}", "sparkles"),       // ✨
        ("\u{1F41B}", "bug"),           // 🐛
        ("\u{267B}\u{FE0F}", "recycle"),// ♻️
        ("\u{267B}", "recycle"),        // ♻ (without variation selector)
        ("\u{26A1}", "zap"),            // ⚡
        ("\u{1F5D1}\u{FE0F}", "wastebasket"), // 🗑️
        ("\u{1F5D1}", "wastebasket"),   // 🗑
        ("\u{1F525}", "fire"),          // 🔥
        ("\u{1F512}", "lock"),          // 🔒
        ("\u{1F4A5}", "boom"),          // 💥
        ("\u{1F4DD}", "memo"),          // 📝
        ("\u{1F527}", "wrench"),        // 🔧
        ("\u{1F477}", "construction_worker"), // 👷
        ("\u{1F49A}", "green_heart"),   // 💚
        ("\u{2705}", "white_check_mark"), // ✅
        ("\u{270F}\u{FE0F}", "pencil2"), // ✏️
        ("\u{270F}", "pencil2"),        // ✏
    ];

    for (emoji_char, emoji_name) in &unicode_map {
        if let Some(rest) = subject.strip_prefix(emoji_char) {
            let desc = rest.trim();
            if !desc.is_empty() {
                return gitmoji_to_parsed(emoji_name, desc);
            }
        }
    }

    None
}

fn gitmoji_to_parsed(emoji: &str, desc: &str) -> Option<ParsedCommit> {
    let (section, breaking) = match emoji {
        "sparkles" => (ChangelogSection::Added, false),
        "bug" => (ChangelogSection::Fixed, false),
        "recycle" | "zap" => (ChangelogSection::Changed, false),
        "wastebasket" => (ChangelogSection::Deprecated, false),
        "fire" => (ChangelogSection::Removed, false),
        "lock" => (ChangelogSection::Security, false),
        "boom" => (ChangelogSection::Changed, true),
        "memo" | "wrench" | "construction_worker" | "green_heart"
        | "white_check_mark" | "pencil2" => return None,
        _ => return None,
    };

    Some(ParsedCommit {
        section,
        scope: None,
        description: capitalize_first(desc),
        breaking,
    })
}

pub fn detect_format(commits: &[CommitInfo]) -> CommitFormatKind {
    let conventional_count = commits.iter()
        .filter(|c| parse_conventional(&c.message).is_some())
        .count();

    let gitmoji_count = commits.iter()
        .filter(|c| parse_gitmoji(&c.message).is_some())
        .count();

    if conventional_count == 0 && gitmoji_count == 0 {
        return CommitFormatKind::Conventional;
    }

    // Conventional wins on tie (more popular)
    if conventional_count >= gitmoji_count {
        CommitFormatKind::Conventional
    } else {
        CommitFormatKind::Gitmoji
    }
}

pub fn parse_commit(message: &str, format: &CommitFormatKind) -> Option<ParsedCommit> {
    match format {
        CommitFormatKind::Conventional => parse_conventional(message),
        CommitFormatKind::Gitmoji => parse_gitmoji(message),
        CommitFormatKind::Auto => unreachable!("Auto should be resolved before parsing"),
    }
}

#[cfg(test)]
mod test {
    use super::*;

    // --- capitalize_first ---

    #[test]
    fn capitalize_first_normal() {
        assert_eq!("Hello world", capitalize_first("hello world"));
    }

    #[test]
    fn capitalize_first_empty() {
        assert_eq!("", capitalize_first(""));
    }

    #[test]
    fn capitalize_first_already_upper() {
        assert_eq!("Hello", capitalize_first("Hello"));
    }

    // --- conventional commits ---

    #[test]
    fn conventional_feat() {
        let result = parse_conventional("feat: add user registration").unwrap();
        assert_eq!(ChangelogSection::Added, result.section);
        assert_eq!("Add user registration", result.description);
        assert!(result.scope.is_none());
        assert!(!result.breaking);
    }

    #[test]
    fn conventional_fix() {
        let result = parse_conventional("fix: resolve timeout error").unwrap();
        assert_eq!(ChangelogSection::Fixed, result.section);
        assert_eq!("Resolve timeout error", result.description);
    }

    #[test]
    fn conventional_with_scope() {
        let result = parse_conventional("feat(auth): add OAuth2 support").unwrap();
        assert_eq!(ChangelogSection::Added, result.section);
        assert_eq!(Some(String::from("auth")), result.scope);
        assert_eq!("Add OAuth2 support", result.description);
    }

    #[test]
    fn conventional_breaking_bang() {
        let result = parse_conventional("feat!: remove legacy API").unwrap();
        assert!(result.breaking);
        assert_eq!(ChangelogSection::Changed, result.section);
        assert_eq!("Remove legacy API", result.description);
    }

    #[test]
    fn conventional_breaking_scope_bang() {
        let result = parse_conventional("refactor(core)!: rewrite engine").unwrap();
        assert!(result.breaking);
        assert_eq!(ChangelogSection::Changed, result.section);
    }

    #[test]
    fn conventional_breaking_in_body() {
        let msg = "feat: new API\n\nBREAKING CHANGE: old API removed";
        let result = parse_conventional(msg).unwrap();
        assert!(result.breaking);
        assert_eq!(ChangelogSection::Changed, result.section);
    }

    #[test]
    fn conventional_breaking_change_hyphen() {
        let msg = "feat: new API\n\nBREAKING-CHANGE: old API removed";
        let result = parse_conventional(msg).unwrap();
        assert!(result.breaking);
    }

    #[test]
    fn conventional_refactor() {
        let result = parse_conventional("refactor: simplify handler").unwrap();
        assert_eq!(ChangelogSection::Changed, result.section);
    }

    #[test]
    fn conventional_perf() {
        let result = parse_conventional("perf: optimize query").unwrap();
        assert_eq!(ChangelogSection::Changed, result.section);
    }

    #[test]
    fn conventional_deprecate() {
        let result = parse_conventional("deprecate: old endpoint").unwrap();
        assert_eq!(ChangelogSection::Deprecated, result.section);
    }

    #[test]
    fn conventional_security() {
        let result = parse_conventional("security: fix XSS vulnerability").unwrap();
        assert_eq!(ChangelogSection::Security, result.section);
    }

    #[test]
    fn conventional_docs_ignored() {
        assert!(parse_conventional("docs: update readme").is_none());
    }

    #[test]
    fn conventional_chore_ignored() {
        assert!(parse_conventional("chore: update deps").is_none());
    }

    #[test]
    fn conventional_ci_ignored() {
        assert!(parse_conventional("ci: add workflow").is_none());
    }

    #[test]
    fn conventional_build_ignored() {
        assert!(parse_conventional("build: update config").is_none());
    }

    #[test]
    fn conventional_style_ignored() {
        assert!(parse_conventional("style: format code").is_none());
    }

    #[test]
    fn conventional_test_ignored() {
        assert!(parse_conventional("test: add unit tests").is_none());
    }

    #[test]
    fn conventional_invalid_format() {
        assert!(parse_conventional("just a random message").is_none());
    }

    #[test]
    fn conventional_unknown_type() {
        assert!(parse_conventional("foobar: something").is_none());
    }

    // --- gitmoji ---

    #[test]
    fn gitmoji_sparkles() {
        let result = parse_gitmoji(":sparkles: add new feature").unwrap();
        assert_eq!(ChangelogSection::Added, result.section);
        assert_eq!("Add new feature", result.description);
        assert!(!result.breaking);
    }

    #[test]
    fn gitmoji_bug() {
        let result = parse_gitmoji(":bug: fix crash on startup").unwrap();
        assert_eq!(ChangelogSection::Fixed, result.section);
    }

    #[test]
    fn gitmoji_recycle() {
        let result = parse_gitmoji(":recycle: refactor auth module").unwrap();
        assert_eq!(ChangelogSection::Changed, result.section);
    }

    #[test]
    fn gitmoji_zap() {
        let result = parse_gitmoji(":zap: improve performance").unwrap();
        assert_eq!(ChangelogSection::Changed, result.section);
    }

    #[test]
    fn gitmoji_wastebasket() {
        let result = parse_gitmoji(":wastebasket: deprecate old API").unwrap();
        assert_eq!(ChangelogSection::Deprecated, result.section);
    }

    #[test]
    fn gitmoji_fire() {
        let result = parse_gitmoji(":fire: remove dead code").unwrap();
        assert_eq!(ChangelogSection::Removed, result.section);
    }

    #[test]
    fn gitmoji_lock() {
        let result = parse_gitmoji(":lock: fix security issue").unwrap();
        assert_eq!(ChangelogSection::Security, result.section);
    }

    #[test]
    fn gitmoji_boom_breaking() {
        let result = parse_gitmoji(":boom: rewrite public API").unwrap();
        assert_eq!(ChangelogSection::Changed, result.section);
        assert!(result.breaking);
    }

    #[test]
    fn gitmoji_memo_ignored() {
        assert!(parse_gitmoji(":memo: update docs").is_none());
    }

    #[test]
    fn gitmoji_wrench_ignored() {
        assert!(parse_gitmoji(":wrench: update config").is_none());
    }

    #[test]
    fn gitmoji_construction_worker_ignored() {
        assert!(parse_gitmoji(":construction_worker: add CI").is_none());
    }

    #[test]
    fn gitmoji_green_heart_ignored() {
        assert!(parse_gitmoji(":green_heart: fix CI").is_none());
    }

    #[test]
    fn gitmoji_white_check_mark_ignored() {
        assert!(parse_gitmoji(":white_check_mark: add tests").is_none());
    }

    #[test]
    fn gitmoji_pencil2_ignored() {
        assert!(parse_gitmoji(":pencil2: fix typo").is_none());
    }

    #[test]
    fn gitmoji_unknown_ignored() {
        assert!(parse_gitmoji(":unknown_emoji: something").is_none());
    }

    #[test]
    fn gitmoji_invalid_format() {
        assert!(parse_gitmoji("just a regular message").is_none());
    }

    // --- auto-detection ---

    #[test]
    fn detect_conventional_majority() {
        let commits = vec![
            CommitInfo { hash: "a".into(), message: "feat: add feature".into(), timestamp: 3 },
            CommitInfo { hash: "b".into(), message: "fix: fix bug".into(), timestamp: 2 },
            CommitInfo { hash: "c".into(), message: ":sparkles: add thing".into(), timestamp: 1 },
        ];
        assert_eq!(CommitFormatKind::Conventional, detect_format(&commits));
    }

    #[test]
    fn detect_gitmoji_majority() {
        let commits = vec![
            CommitInfo { hash: "a".into(), message: ":sparkles: add feature".into(), timestamp: 3 },
            CommitInfo { hash: "b".into(), message: ":bug: fix bug".into(), timestamp: 2 },
            CommitInfo { hash: "c".into(), message: "feat: add thing".into(), timestamp: 1 },
        ];
        assert_eq!(CommitFormatKind::Gitmoji, detect_format(&commits));
    }

    #[test]
    fn detect_tie_picks_conventional() {
        let commits = vec![
            CommitInfo { hash: "a".into(), message: "feat: add feature".into(), timestamp: 2 },
            CommitInfo { hash: "b".into(), message: ":bug: fix bug".into(), timestamp: 1 },
        ];
        assert_eq!(CommitFormatKind::Conventional, detect_format(&commits));
    }

    #[test]
    fn detect_no_matches_defaults_to_conventional() {
        let commits = vec![
            CommitInfo { hash: "a".into(), message: "random commit".into(), timestamp: 1 },
        ];
        assert_eq!(CommitFormatKind::Conventional, detect_format(&commits));
    }

    #[test]
    fn detect_empty_commits() {
        assert_eq!(CommitFormatKind::Conventional, detect_format(&[]));
    }

    // --- ChangelogSection::from_header ---

    #[test]
    fn section_from_header_added() {
        assert_eq!(Some(ChangelogSection::Added), ChangelogSection::from_header("Added"));
    }

    #[test]
    fn section_from_header_add() {
        assert_eq!(Some(ChangelogSection::Added), ChangelogSection::from_header("Add"));
    }

    #[test]
    fn section_from_header_modified() {
        assert_eq!(Some(ChangelogSection::Changed), ChangelogSection::from_header("Modified"));
    }

    #[test]
    fn section_from_header_fix() {
        assert_eq!(Some(ChangelogSection::Fixed), ChangelogSection::from_header("Fix"));
    }

    #[test]
    fn section_from_header_unknown() {
        assert!(ChangelogSection::from_header("Unknown").is_none());
    }
}
