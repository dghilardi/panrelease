use anyhow::{anyhow, bail};
use crate::args::parse_build;

#[derive(Debug, PartialEq)]
pub enum BranchKind {
    Mainline,
    Feature { slug: String },
    Hotfix { slug: String },
}

const FEATURE_PREFIXES: &[&str] = &["feat/", "feature/"];
const HOTFIX_PREFIXES: &[&str] = &["hotfix/", "fix/"];

pub fn classify_branch(branch: &str, mainline: &str) -> anyhow::Result<BranchKind> {
    if branch == mainline {
        return Ok(BranchKind::Mainline);
    }

    for prefix in FEATURE_PREFIXES {
        if let Some(remainder) = branch.strip_prefix(prefix) {
            let slug = sanitize_slug(remainder);
            if slug.is_empty() {
                bail!("Strict mode: branch '{branch}' has an empty name after the prefix");
            }
            return Ok(BranchKind::Feature { slug });
        }
    }

    for prefix in HOTFIX_PREFIXES {
        if let Some(remainder) = branch.strip_prefix(prefix) {
            let slug = sanitize_slug(remainder);
            if slug.is_empty() {
                bail!("Strict mode: branch '{branch}' has an empty name after the prefix");
            }
            return Ok(BranchKind::Hotfix { slug: format!("fix-{slug}") });
        }
    }

    bail!(
        "Strict mode: branch '{branch}' does not match any known pattern \
         (mainline, feat/*, feature/*, hotfix/*, fix/*). \
         Strict mode rejects releases from unknown branches."
    )
}

pub fn sanitize_slug(raw: &str) -> String {
    let replaced: String = raw
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect();

    let collapsed = replaced
        .split('-')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("-");

    collapsed
}

pub fn version_from_tag(tag: &str, tag_template: &str) -> Option<semver::Version> {
    let parts: Vec<&str> = tag_template.splitn(2, "{{version}}").collect();
    if parts.len() != 2 {
        return None;
    }
    let prefix = parts[0];
    let suffix = parts[1];

    let stripped = tag.strip_prefix(prefix)?;
    let version_str = if suffix.is_empty() {
        stripped
    } else {
        stripped.strip_suffix(suffix)?
    };

    semver::Version::parse(version_str).ok()
}

pub fn validate_release(
    branch_kind: &BranchKind,
    new_version: &semver::Version,
    base_version: Option<&semver::Version>,
) -> anyhow::Result<()> {
    match branch_kind {
        BranchKind::Mainline => {
            if !new_version.build.is_empty() {
                bail!(
                    "Strict mode: cannot release a post-version ({new_version}) from mainline. \
                     Use major, minor, or patch."
                );
            }
        }
        BranchKind::Feature { slug } | BranchKind::Hotfix { slug } => {
            let kind_name = if matches!(branch_kind, BranchKind::Feature { .. }) {
                "feature"
            } else {
                "hotfix"
            };

            if new_version.build.is_empty() {
                bail!(
                    "Strict mode: {kind_name} branches can only produce post-releases. \
                     Use 'post' bump level instead of major/minor/patch."
                );
            }

            let base = base_version.ok_or_else(|| {
                anyhow!(
                    "Strict mode: no version tag found reachable from merge-base with mainline"
                )
            })?;

            if !base.build.is_empty() {
                bail!(
                    "Strict mode: mainline has a post-version ({base}) at the branch point. \
                     Release a clean version from mainline first, then rebase this branch."
                );
            }

            if new_version.major != base.major
                || new_version.minor != base.minor
                || new_version.patch != base.patch
            {
                bail!(
                    "Strict mode: version base {}.{}.{} does not match branch base {}.{}.{}. \
                     Rebase to latest mainline and remove intermediate releases.",
                    new_version.major, new_version.minor, new_version.patch,
                    base.major, base.minor, base.patch,
                );
            }

            let build_str = new_version.build.as_str();
            if let Some((name, _)) = parse_build(build_str) {
                if name != slug {
                    bail!(
                        "Strict mode: build metadata slug '{name}' does not match \
                         expected slug '{slug}' for this branch."
                    );
                }
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod test {
    use super::*;

    // --- sanitize_slug ---

    #[test]
    fn sanitize_normal_input() {
        assert_eq!("user-registration", sanitize_slug("user-registration"));
    }

    #[test]
    fn sanitize_special_chars() {
        assert_eq!("hello-world", sanitize_slug("hello@.-world"));
    }

    #[test]
    fn sanitize_consecutive_specials() {
        assert_eq!("a-b", sanitize_slug("a@#$%b"));
    }

    #[test]
    fn sanitize_leading_trailing() {
        assert_eq!("abc", sanitize_slug("--abc--"));
    }

    #[test]
    fn sanitize_all_special() {
        assert_eq!("", sanitize_slug("@#$%"));
    }

    #[test]
    fn sanitize_preserves_case() {
        assert_eq!("NPE-during-registration", sanitize_slug("NPE during registration"));
    }

    // --- classify_branch ---

    #[test]
    fn classify_mainline() {
        let result = classify_branch("main", "main").unwrap();
        assert_eq!(BranchKind::Mainline, result);
    }

    #[test]
    fn classify_feat_prefix() {
        let result = classify_branch("feat/user-reg", "main").unwrap();
        assert_eq!(BranchKind::Feature { slug: String::from("user-reg") }, result);
    }

    #[test]
    fn classify_feature_prefix() {
        let result = classify_branch("feature/user-reg", "main").unwrap();
        assert_eq!(BranchKind::Feature { slug: String::from("user-reg") }, result);
    }

    #[test]
    fn classify_hotfix_prefix() {
        let result = classify_branch("hotfix/npe-fix", "main").unwrap();
        assert_eq!(BranchKind::Hotfix { slug: String::from("fix-npe-fix") }, result);
    }

    #[test]
    fn classify_fix_prefix() {
        let result = classify_branch("fix/timeout", "main").unwrap();
        assert_eq!(BranchKind::Hotfix { slug: String::from("fix-timeout") }, result);
    }

    #[test]
    fn classify_unknown_branch() {
        let result = classify_branch("develop", "main");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("does not match any known pattern"));
    }

    #[test]
    fn classify_feature_sanitizes_slug() {
        let result = classify_branch("feature/hello@.-world", "main").unwrap();
        assert_eq!(BranchKind::Feature { slug: String::from("hello-world") }, result);
    }

    #[test]
    fn classify_empty_after_prefix() {
        let result = classify_branch("feat/", "main");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("empty name"));
    }

    // --- version_from_tag ---

    #[test]
    fn version_from_default_template() {
        let v = version_from_tag("1.2.3", "{{version}}").unwrap();
        assert_eq!(semver::Version::parse("1.2.3").unwrap(), v);
    }

    #[test]
    fn version_from_v_prefix_template() {
        let v = version_from_tag("v1.2.3", "v{{version}}").unwrap();
        assert_eq!(semver::Version::parse("1.2.3").unwrap(), v);
    }

    #[test]
    fn version_from_complex_template() {
        let v = version_from_tag("release-1.2.3", "release-{{version}}").unwrap();
        assert_eq!(semver::Version::parse("1.2.3").unwrap(), v);
    }

    #[test]
    fn version_from_tag_with_suffix_template() {
        let v = version_from_tag("v1.2.3-rel", "v{{version}}-rel").unwrap();
        assert_eq!(semver::Version::parse("1.2.3").unwrap(), v);
    }

    #[test]
    fn version_from_tag_invalid_tag() {
        assert!(version_from_tag("not-a-version", "{{version}}").is_none());
    }

    #[test]
    fn version_from_tag_no_placeholder() {
        assert!(version_from_tag("1.2.3", "no-placeholder").is_none());
    }

    #[test]
    fn version_from_tag_with_build_metadata() {
        let v = version_from_tag("1.2.3+feat.r1", "{{version}}").unwrap();
        assert_eq!(semver::Version::parse("1.2.3+feat.r1").unwrap(), v);
    }

    // --- validate_release ---

    #[test]
    fn validate_mainline_clean_version_ok() {
        let v = semver::Version::parse("1.3.0").unwrap();
        assert!(validate_release(&BranchKind::Mainline, &v, None).is_ok());
    }

    #[test]
    fn validate_mainline_post_version_rejected() {
        let v = semver::Version::parse("1.3.0+feat.r1").unwrap();
        let result = validate_release(&BranchKind::Mainline, &v, None);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("cannot release a post-version"));
    }

    #[test]
    fn validate_feature_post_matching_ok() {
        let new = semver::Version::parse("1.3.7+user-reg.r1").unwrap();
        let base = semver::Version::parse("1.3.7").unwrap();
        let kind = BranchKind::Feature { slug: String::from("user-reg") };
        assert!(validate_release(&kind, &new, Some(&base)).is_ok());
    }

    #[test]
    fn validate_feature_clean_version_rejected() {
        let new = semver::Version::parse("1.4.0").unwrap();
        let base = semver::Version::parse("1.3.7").unwrap();
        let kind = BranchKind::Feature { slug: String::from("user-reg") };
        let result = validate_release(&kind, &new, Some(&base));
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("can only produce post-releases"));
    }

    #[test]
    fn validate_feature_wrong_slug_rejected() {
        let new = semver::Version::parse("1.3.7+other-feat.r1").unwrap();
        let base = semver::Version::parse("1.3.7").unwrap();
        let kind = BranchKind::Feature { slug: String::from("user-reg") };
        let result = validate_release(&kind, &new, Some(&base));
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("does not match expected slug"));
    }

    #[test]
    fn validate_feature_wrong_base_rejected() {
        let new = semver::Version::parse("1.3.8+user-reg.r1").unwrap();
        let base = semver::Version::parse("1.3.7").unwrap();
        let kind = BranchKind::Feature { slug: String::from("user-reg") };
        let result = validate_release(&kind, &new, Some(&base));
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("does not match branch base"));
    }

    #[test]
    fn validate_feature_dirty_base_rejected() {
        let new = semver::Version::parse("1.3.7+user-reg.r1").unwrap();
        let base = semver::Version::parse("1.3.7+old-feat.r3").unwrap();
        let kind = BranchKind::Feature { slug: String::from("user-reg") };
        let result = validate_release(&kind, &new, Some(&base));
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("post-version"));
        assert!(err_msg.contains("Release a clean version from mainline"));
    }

    #[test]
    fn validate_feature_no_base_rejected() {
        let new = semver::Version::parse("1.3.7+user-reg.r1").unwrap();
        let kind = BranchKind::Feature { slug: String::from("user-reg") };
        let result = validate_release(&kind, &new, None);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("no version tag found"));
    }

    #[test]
    fn validate_hotfix_post_matching_ok() {
        let new = semver::Version::parse("1.3.7+fix-timeout.r1").unwrap();
        let base = semver::Version::parse("1.3.7").unwrap();
        let kind = BranchKind::Hotfix { slug: String::from("fix-timeout") };
        assert!(validate_release(&kind, &new, Some(&base)).is_ok());
    }

    #[test]
    fn validate_hotfix_wrong_slug_rejected() {
        let new = semver::Version::parse("1.3.7+no-fix-prefix.r1").unwrap();
        let base = semver::Version::parse("1.3.7").unwrap();
        let kind = BranchKind::Hotfix { slug: String::from("fix-timeout") };
        let result = validate_release(&kind, &new, Some(&base));
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("does not match expected slug"));
    }
}
