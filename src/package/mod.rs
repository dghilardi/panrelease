use std::path::{Path, PathBuf};
use crate::package::cargo::CargoPackage;

pub mod cargo;
pub mod npm;
pub mod maven;

pub trait PanPackage {
    fn extract_version(&self) -> anyhow::Result<semver::Version>;
    fn set_version(&mut self, version: &semver::Version) -> anyhow::Result<()>;
    fn persist(&self) -> anyhow::Result<()>;
    fn hook_after_rel(&self) -> anyhow::Result<()>;
}