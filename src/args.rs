use std::path::PathBuf;
use std::str::FromStr;

use clap::{Args, Parser, Subcommand, ValueEnum};
use regex::Regex;
use semver::{BuildMetadata, Prerelease};

/// Simple program release and tag software versions
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct PanReleaseArgs {
    #[arg(short, long)]
    pub path: Option<PathBuf>,
    #[clap(subcommand)]
    pub subcommand: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Release a new version
    Release(RelArgs),
}

#[derive(Args, Debug)]
pub struct RelArgs {
    /// Either bump by LEVEL or set the VERSION for all selected packages
    #[arg(value_name = "LEVEL|VERSION", help_heading = "Version")]
    pub level_or_version: TargetVersion,
}

#[derive(Clone, Debug)]
pub enum TargetVersion {
    Relative(BumpLevel),
    Absolute(semver::Version),
}

impl TargetVersion {
    pub fn apply(
        &self,
        current: semver::Version,
    ) -> semver::Version {
        match self {
            TargetVersion::Relative(bump_level) => {
                bump_level.apply(current)
            }
            TargetVersion::Absolute(version) => {
                version.to_owned()
            }
        }
    }
}

impl clap::builder::ValueParserFactory for TargetVersion {
    type Parser = TargetVersionParser;

    fn value_parser() -> Self::Parser {
        TargetVersionParser
    }
}

impl std::str::FromStr for TargetVersion {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if let Ok(bump_level) = BumpLevel::from_str(s, false) {
            Ok(TargetVersion::Relative(bump_level))
        } else {
            Ok(TargetVersion::Absolute(
                semver::Version::parse(s).map_err(|e| e.to_string())?,
            ))
        }
    }
}

#[derive(Copy, Clone)]
pub struct TargetVersionParser;

impl clap::builder::TypedValueParser for TargetVersionParser {
    type Value = TargetVersion;

    fn parse_ref(
        &self,
        cmd: &clap::Command,
        arg: Option<&clap::Arg>,
        value: &std::ffi::OsStr,
    ) -> Result<Self::Value, clap::Error> {
        let inner_parser = TargetVersion::from_str;
        inner_parser.parse_ref(cmd, arg, value)
    }

    fn possible_values(
        &self,
    ) -> Option<Box<dyn Iterator<Item=clap::builder::PossibleValue> + '_>> {
        let inner_parser = clap::builder::EnumValueParser::<BumpLevel>::new();
        #[allow(clippy::needless_collect)] // Erasing a lifetime
        inner_parser.possible_values().map(|ps| {
            let ps = ps.collect::<Vec<_>>();
            let ps: Box<dyn Iterator<Item=clap::builder::PossibleValue> + '_> =
                Box::new(ps.into_iter());
            ps
        })
    }
}

#[derive(ValueEnum, Debug, Clone, Copy)]
#[value(rename_all = "kebab-case")]
pub enum BumpLevel {
    Major,
    Minor,
    Patch,
    Post,
}

impl BumpLevel {
    fn apply(
        &self,
        current: semver::Version,
    ) -> semver::Version {
        match self {
            BumpLevel::Major => {
                semver::Version {
                    major: current.major + 1,
                    minor: 0,
                    patch: 0,
                    pre: Default::default(),
                    build: Default::default(),
                }
            }
            BumpLevel::Minor => {
                semver::Version {
                    major: current.major,
                    minor: current.minor + 1,
                    patch: 0,
                    pre: Default::default(),
                    build: Default::default(),
                }
            }
            BumpLevel::Patch => {
                semver::Version {
                    major: current.major,
                    minor: current.minor,
                    patch: current.patch + 1,
                    pre: Default::default(),
                    build: Default::default(),
                }
            }
            BumpLevel::Post => {
                let build = parse_build(current.build.as_str()).map(|(name, ver)| {
                    BuildMetadata::new(&format!("{}.r{}", name, ver.map(|v| v + 1).unwrap_or(1)))
                })
                    .unwrap_or_else(|| BuildMetadata::new("dev.r1"))
                    .expect("Error constructing post-release slug");

                semver::Version {
                    major: current.major,
                    minor: current.minor,
                    patch: current.patch,
                    pre: Default::default(),
                    build,
                }
            }
        }
    }
}

fn parse_build(build_info: &str) -> Option<(&str, Option<u64>)> {
    let re = Regex::new(r"(?P<name>[0-9a-zA-Z-]+(?:\.[0-9a-zA-Z-]+)*)(\.r(?P<ver>\d+))$").unwrap();
    re.captures(build_info)
        .and_then(|captures| {
            let ver = captures.name("ver")
                .and_then(|m| m.as_str().parse::<u64>().ok());
            captures.name("name").map(|name| (name.as_str(), ver))
        })
        .or(Some((build_info, None)))
        .filter(|(name, ver)| !name.is_empty() || ver.is_some())
}

#[cfg(test)]
mod test {
    use semver::BuildMetadata;
    use crate::args::{BumpLevel, parse_build};

    #[test]
    fn parse_simple_post_release() {
        assert_eq!(Some(("build", Some(12))), parse_build("build.r12"))
    }

    #[test]
    fn parse_no_ver_post_release() {
        assert_eq!(Some(("build", None)), parse_build("build"))
    }

    #[test]
    fn parse_complex_post_release() {
        assert_eq!(Some(("feature-dev.rc", Some(12))), parse_build("feature-dev.rc.r012"))
    }

    #[test]
    fn parse_invalid_post_release() {
        assert_eq!(Some(("build.rc12", None)), parse_build("build.rc12"))
    }

    #[test]
    fn increment_patch() {
        assert_eq!(
            String::from("1.2.4"),
            BumpLevel::Patch.apply(semver::Version::parse("1.2.3").unwrap()).to_string()
        )
    }

    #[test]
    fn increment_minor() {
        assert_eq!(
            String::from("1.3.0"),
            BumpLevel::Minor.apply(semver::Version::parse("1.2.3").unwrap()).to_string()
        )
    }

    #[test]
    fn increment_major() {
        assert_eq!(
            String::from("2.0.0"),
            BumpLevel::Major.apply(semver::Version::parse("1.2.3").unwrap()).to_string()
        )
    }

    #[test]
    fn increment_postrel() {
        assert_eq!(
            String::from("1.2.3+feat.r2"),
            BumpLevel::Post.apply(semver::Version::parse("1.2.3+feat.r1").unwrap()).to_string()
        )
    }
}