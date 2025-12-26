mod plain_text_spec;
mod error;
mod assertions;

pub use error::{Error, Result, ResponseChecker};

pub enum TestCases {
    PlainText,
}