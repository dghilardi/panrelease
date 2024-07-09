use std::marker::PhantomData;
use std::path::PathBuf;

use anyhow::anyhow;
use regex::Regex;
use semver::Version;

use crate::package::PanPackage;
use crate::system::FileSystem;

pub struct GradlePackage<F> {
    path: PathBuf,
    properties: String,
    filesystem: PhantomData<F>,
}

impl<F: FileSystem> GradlePackage<F> {
    pub fn new(path: PathBuf) -> anyhow::Result<Self> {
        let properties = F::read_string(&path.join("gradle.properties"))?;
        Ok(Self {
            path,
            properties,
            filesystem: PhantomData,
        })
    }
}

const VERSION_REGEX: &str = r#"(?P<version_prefix>(^|\n)version\s*=\s*)(?P<version_value>\d\S+)"#;

impl<F: FileSystem> PanPackage for GradlePackage<F> {
    fn extract_version(&self) -> anyhow::Result<Version> {
        let version_regex = Regex::new(VERSION_REGEX)?;
        let version_str = version_regex
            .captures(&self.properties)
            .and_then(|cap| cap.name("version_value"))
            .ok_or_else(|| anyhow!("Could not find version in gradle.properties"))?
            .as_str();

        Ok(Version::parse(version_str)?)
    }

    fn set_version(&mut self, version: &Version) -> anyhow::Result<()> {
        let version_regex = Regex::new(VERSION_REGEX)?;
        self.properties = version_regex
            .replace(
                &self.properties,
                format!("${{version_prefix}}{}", version.to_string()),
            )
            .to_string();

        Ok(())
    }

    fn persist(&self) -> anyhow::Result<()> {
        F::write_string(&self.path.join("gradle.properties"), &self.properties)?;
        Ok(())
    }

    fn hook_after_rel(&self) -> anyhow::Result<()> {
        Ok(())
    }
}
