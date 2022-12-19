pub mod json;
pub mod xml;

pub trait FormatCodec {
    fn extract(&self, path: &str) -> anyhow::Result<Option<&str>>;
    fn replace(&mut self, path: &str, value: &str) -> anyhow::Result<()>;
}