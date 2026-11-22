use std::ops::Range;
use std::path::{Component, Path, PathBuf};

use anyhow::{ensure, Context, Result};
use serde::Deserialize;
use yaml_edit::{Document, Mapping, YamlNode};

use crate::git::GitRepo;
use crate::system::FileSystem;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct DesignDocsConfig {
    service: String,
    #[serde(default = "default_path")]
    path: PathBuf,
    #[serde(default)]
    format: DesignDocsFormat,
}

fn default_path() -> PathBuf {
    PathBuf::from("docs/design")
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "kebab-case")]
enum DesignDocsFormat {
    #[default]
    CloudDesignV1,
}

struct Design {
    status: String,
    service: String,
    tracking: String,
    implementation: String,
    release_slot: Option<Range<usize>>,
}

impl DesignDocsFormat {
    fn supports(&self, path: &str) -> bool {
        match self {
            Self::CloudDesignV1 => matches!(
                Path::new(path)
                    .extension()
                    .and_then(|extension| extension.to_str()),
                Some("md" | "mdx")
            ),
        }
    }

    fn parse(&self, source: &str) -> Result<Option<Design>> {
        match self {
            Self::CloudDesignV1 => parse_cloud_design(source),
        }
    }
}

fn parse_cloud_design(source: &str) -> Result<Option<Design>> {
    let Some((opening, rest)) = source.split_once('\n') else {
        return Ok(None);
    };
    if opening.trim_start_matches('\u{feff}').trim_end() != "---" {
        return Ok(None);
    }
    let mut end = 0;
    for line in rest.split_inclusive('\n') {
        if line.trim_end() == "---" {
            break;
        }
        end += line.len();
    }
    ensure!(end < rest.len(), "unterminated YAML front matter");

    let doc: Document = rest[..end].parse()?;
    let Some(root) = doc.as_mapping() else {
        return Ok(None);
    };
    let Some(delivery) = unique_value(&root, "delivery")? else {
        return Ok(None);
    };
    let Some(delivery) = delivery.as_mapping() else {
        return Ok(None);
    };
    let (Some(status), Some(service), Some(tracking), Some(implementation)) = (
        string_value(&root, "status")?,
        string_value(delivery, "service")?,
        string_value(delivery, "tracking")?,
        string_value(delivery, "implementation")?,
    ) else {
        return Ok(None);
    };

    let release_slot = unique_value(delivery, "released_in")?.and_then(|value| {
        let range = value.byte_range();
        let token = &rest[range.start as usize..range.end as usize];
        // Replace a direct null token, preserving all surrounding YAML and Markdown.
        matches!(token, "null" | "Null" | "NULL" | "~").then(|| {
            let offset = opening.len() + 1;
            (offset + range.start as usize)..(offset + range.end as usize)
        })
    });
    Ok(Some(Design {
        status,
        service,
        tracking,
        implementation,
        release_slot,
    }))
}

fn unique_value(mapping: &Mapping, key: &str) -> Result<Option<YamlNode>> {
    let mut entries = mapping.entries().filter(|entry| entry.key_matches(key));
    let value = entries.next().and_then(|entry| entry.value_node());
    ensure!(entries.next().is_none(), "duplicate front matter key {key}");
    Ok(value)
}

fn string_value(mapping: &Mapping, key: &str) -> Result<Option<String>> {
    Ok(unique_value(mapping, key)?.and_then(|node| {
        let scalar = node.as_scalar()?;
        (!scalar.is_null()).then(|| scalar.as_string())
    }))
}

pub(super) struct DesignUpdates {
    revision: String,
    edits: Vec<DesignEdit>,
}

struct DesignEdit {
    path: PathBuf,
    original: String,
    release_slot: Range<usize>,
}

impl DesignUpdates {
    pub(super) fn apply<F: FileSystem>(
        self,
        repo: &GitRepo,
        version: &semver::Version,
    ) -> Result<()> {
        if self.edits.is_empty() {
            return Ok(());
        }
        ensure!(
            repo.head_revision()? == self.revision,
            "HEAD changed during release"
        );
        for edit in &self.edits {
            ensure!(
                F::read_string(&edit.path)? == edit.original,
                "Design document {:?} changed during release",
                edit.path
            );
        }
        for mut edit in self.edits {
            edit.original
                .replace_range(edit.release_slot, &format!("\"{version}\""));
            F::write_string(&edit.path, &edit.original)
                .with_context(|| format!("Could not update {:?}", edit.path))?;
        }
        Ok(())
    }
}

pub(super) fn plan<F: FileSystem>(
    repo: &GitRepo,
    root: &Path,
    config: &DesignDocsConfig,
) -> Result<DesignUpdates> {
    ensure!(
        !config.service.trim().is_empty(),
        "design_docs.service must be nonempty"
    );
    let mut parts = Vec::new();
    for component in config.path.components() {
        match component {
            Component::Normal(part) => {
                parts.push(part.to_str().context("Invalid design_docs.path")?)
            }
            Component::CurDir => {}
            _ => anyhow::bail!(
                "design_docs.path must be a directory relative to the repository root"
            ),
        }
    }
    ensure!(!parts.is_empty(), "design_docs.path must name a directory");

    let revision = repo.head_revision()?;
    let mut edits = Vec::new();
    let directory = parts.join("/");
    let prefix = format!("{directory}/");
    let blobs = repo
        .tracked_blobs(&revision, &directory)?
        .into_iter()
        .filter(|(path, _)| path.starts_with(&prefix) && config.format.supports(path))
        .collect::<Vec<_>>();
    if blobs.is_empty() {
        return Ok(DesignUpdates { revision, edits });
    }
    let base = repo.stable_release_base(&revision)?;
    if base.is_none() && repo.is_shallow()? {
        warn("Skipping design documents because shallow Git history has no previous stable release tag");
        return Ok(DesignUpdates { revision, edits });
    }

    for (path, oid) in blobs {
        let committed = repo.read_blob(&oid)?;
        let design = match config.format.parse(&committed) {
            Ok(Some(design)) => design,
            Ok(None) => continue,
            Err(error) => {
                warn(&format!("Skipping design document {path}: {error}"));
                continue;
            }
        };
        if design.status != "accepted"
            || design.service != config.service
            || design.implementation != "implemented"
            || design.release_slot.is_none()
        {
            continue;
        }
        if !has_implementation_transition(
            repo,
            &config.format,
            &revision,
            base.as_deref(),
            &path,
            &design,
        )? {
            warn(&format!("Skipping design document {path}: no verified Git transition to implemented for this release"));
            continue;
        }
        let path = root.join(path);
        let original = F::read_string(&path)?;
        ensure!(
            original == committed || original.replace("\r\n", "\n") == committed,
            "Design document {path:?} differs from the revision being released"
        );
        let release_slot = config
            .format
            .parse(&original)?
            .and_then(|design| design.release_slot)
            .context("Could not locate released_in in the working tree")?;
        edits.push(DesignEdit {
            path,
            original,
            release_slot,
        });
    }
    Ok(DesignUpdates { revision, edits })
}

fn has_implementation_transition(
    repo: &GitRepo,
    format: &DesignDocsFormat,
    revision: &str,
    base: Option<&str>,
    path: &str,
    current: &Design,
) -> Result<bool> {
    let at = |revision| -> Result<Option<Design>> {
        let blobs = repo.tracked_blobs(revision, path)?;
        let Some((_, oid)) = blobs.into_iter().find(|(file, _)| file == path) else {
            return Ok(None);
        };
        let source = repo.read_blob(&oid)?;
        // A malformed historical snapshot cannot establish an implementation transition.
        Ok(format.parse(&source).ok().flatten())
    };
    let same_design =
        |design: &Design| design.tracking == current.tracking && design.service == current.service;
    let previous_release = base.map(at).transpose()?.flatten();
    if previous_release.as_ref().is_some_and(|design| {
        same_design(design)
            && (design.implementation == "implemented" || design.release_slot.is_none())
    }) {
        return Ok(false);
    }

    let revisions = repo.path_revisions(revision, base, path)?;
    for commit in revisions.iter().skip(1).map(String::as_str).chain(base) {
        let Some(previous) = at(commit)?.filter(same_design) else {
            return Ok(false);
        };
        if previous.release_slot.is_none() {
            return Ok(false);
        }
        match previous.implementation.as_str() {
            "implemented" => {}
            "not_started" | "in_progress" => return Ok(true),
            _ => return Ok(false),
        }
    }
    Ok(false)
}

fn warn(message: &str) {
    #[cfg(not(target_arch = "wasm32"))]
    eprintln!("{message}");
    #[cfg(target_arch = "wasm32")]
    crate::wasm_utils::log(message);
}
