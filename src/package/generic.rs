use std::marker::PhantomData;
use std::path::PathBuf;

use anyhow::anyhow;
use semver::Version;

use crate::package::PanPackage;
use crate::parser::FormatCodec;
use crate::parser::json::JsonString;
use crate::parser::tomlcodec::TomlString;
use crate::parser::xml::xmlstring::XmlString;
use crate::project::config::GenericFormat;
use crate::system::FileSystem;

fn translate_path(user_path: &str, format: &GenericFormat) -> String {
    match format {
        GenericFormat::Xml => user_path.replace('.', "/"),
        GenericFormat::Json | GenericFormat::Toml => user_path.to_string(),
    }
}

enum Codec {
    Json(JsonString),
    Xml(XmlString),
    Toml(TomlString),
}

pub struct GenericPackage<F> {
    file_path: PathBuf,
    codec: Codec,
    version_path: String,
    filesystem: PhantomData<F>,
}

impl<F: FileSystem> GenericPackage<F> {
    pub fn new(
        module_path: PathBuf,
        file: &str,
        format: &GenericFormat,
        version_field: &str,
    ) -> anyhow::Result<Self> {
        let file_path = module_path.join(file);
        let content = F::read_string(&file_path)?;
        let version_path = translate_path(version_field, format);

        let codec = match format {
            GenericFormat::Json => Codec::Json(JsonString::new(&content)),
            GenericFormat::Xml => Codec::Xml(XmlString::new(&content)),
            GenericFormat::Toml => Codec::Toml(TomlString::new(&content)?),
        };

        Ok(Self {
            file_path,
            codec,
            version_path,
            filesystem: PhantomData,
        })
    }
}

impl<F: FileSystem> PanPackage for GenericPackage<F> {
    fn extract_version(&self) -> anyhow::Result<Version> {
        let version_str = match &self.codec {
            Codec::Json(c) => c.extract(&self.version_path)?,
            Codec::Xml(c) => c.extract(&self.version_path)?,
            Codec::Toml(c) => c.extract(&self.version_path)?,
        };
        let version_str = version_str.ok_or_else(|| {
            anyhow!(
                "Could not find version at path '{}' in {:?}",
                self.version_path,
                self.file_path
            )
        })?;
        Ok(Version::parse(version_str)?)
    }

    fn set_version(&mut self, version: &Version) -> anyhow::Result<()> {
        match &mut self.codec {
            Codec::Json(c) => c.replace(&self.version_path, &version.to_string())?,
            Codec::Xml(c) => c.replace(&self.version_path, &version.to_string())?,
            Codec::Toml(c) => c.replace(&self.version_path, &version.to_string())?,
        }
        Ok(())
    }

    fn persist(&self) -> anyhow::Result<()> {
        let content = match &self.codec {
            Codec::Json(c) => c.to_string(),
            Codec::Xml(c) => c.to_string(),
            Codec::Toml(c) => c.to_string(),
        };
        F::write_string(&self.file_path, &content)?;
        Ok(())
    }

    fn hook_after_rel(&self) -> anyhow::Result<()> {
        Ok(())
    }
}
