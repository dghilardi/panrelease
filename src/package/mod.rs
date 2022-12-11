pub mod cargo;

pub trait PanPackage {
    fn extract_version(&self) -> anyhow::Result<semver::Version>;
    fn set_version(&mut self, version: &semver::Version) -> anyhow::Result<()>;
    fn persist(&self) -> anyhow::Result<()>;
}