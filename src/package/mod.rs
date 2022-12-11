pub mod cargo;

pub trait PanPackage {
    fn extract_version(&self) -> anyhow::Result<semver::Version>;
}