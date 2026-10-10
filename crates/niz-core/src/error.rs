#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Invalid configuration: {0}")]
    Invalid(String),
    #[error("Unsupported configuration: {0}")]
    Unsupported(String),
    #[error("JSON: {0}")]
    Json(#[from] serde_json::Error),
    #[error("XML: {0}")]
    Xml(#[from] quick_xml::Error),
}
pub type Result<T> = std::result::Result<T, Error>;
pub(crate) fn require(condition: bool, message: impl Into<String>) -> Result<()> {
    if condition {
        Ok(())
    } else {
        Err(Error::Invalid(message.into()))
    }
}
