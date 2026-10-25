pub mod commands;
pub mod component_feed;
pub mod component_update;
pub mod feed;
pub mod layout;
pub mod manifest;
pub mod paths;
#[cfg(test)]
mod tests;
pub mod transfer;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
pub struct Error(pub String);
impl std::fmt::Display for Error {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}
impl std::error::Error for Error {}
impl From<std::io::Error> for Error {
    fn from(error: std::io::Error) -> Self {
        Self(error.to_string())
    }
}
impl From<serde_json::Error> for Error {
    fn from(error: serde_json::Error) -> Self {
        Self(error.to_string())
    }
}
impl From<reqwest::Error> for Error {
    fn from(error: reqwest::Error) -> Self {
        Self(error.to_string())
    }
}
