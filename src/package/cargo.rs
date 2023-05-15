use std::marker::PhantomData;
use std::path::PathBuf;
use std::str::FromStr;

use semver::Version;

use crate::package::PanPackage;
use crate::runner::CmdRunner;
use crate::system::FileSystem;

pub struct CargoPackage<F> {
    path: PathBuf,
    doc: toml_edit::Document,
    filesystem: PhantomData<F>,
}

impl <F: FileSystem> CargoPackage<F> {
    pub fn new(path: PathBuf) -> anyhow::Result<Self> {
        let cargo_str = F::read_string(&path.join("Cargo.toml"))?;
        Ok(Self {
            path,
            doc: cargo_str.parse::<toml_edit::Document>()?,
            filesystem: PhantomData,
        })
    }
}

impl <F: FileSystem> PanPackage for CargoPackage<F> {
    fn extract_version(&self) -> anyhow::Result<Version> {
        let Some(ver) = self.doc["package"]["version"].as_str() else {
            anyhow::bail!("cannot find version in Cargo.toml")
        };
        Ok(Version::from_str(ver)?)
    }

    fn set_version(&mut self, version: &Version) -> anyhow::Result<()> {
        self.doc["package"]["version"] = toml_edit::value(version.to_string());
        Ok(())
    }

    fn persist(&self) -> anyhow::Result<()> {
        F::write_string(&self.path.join("Cargo.toml"), &self.doc.to_string())?;
        Ok(())
    }

    fn hook_after_rel(&self) -> anyhow::Result<()> {
        let mut runner = CmdRunner::build("cargo", &[String::from("check")], &self.path)?;
        runner.run()?;
        Ok(())
    }
}