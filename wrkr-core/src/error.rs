use thiserror::Error;


#[derive(Error, Debug)]
pub enum Error {
    #[error("Lua error: {0}")]
    LuaError(#[from] mlua::Error),
    #[error("HTTP error: {0}")]
    HttpError(#[from] reqwest::Error),
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
    #[error("Other error: {0}")]
    Other(String),
}

pub type Result<T> = std::result::Result<T, Error>;