use std::fmt;
use std::fmt::Display;

use serde::{de, ser};

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Clone)]
pub enum Error {
    Message(String),
    ExpectedStanzaEnd,
    MissingColon(usize),
}

impl ser::Error for Error {
    fn custom<T: Display>(msg: T) -> Self { Error::Message(msg.to_string()) }
}
impl de::Error for Error {
    fn custom<T: Display>(msg: T) -> Self { Error::Message(msg.to_string()) }
}

impl Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result { write!(f, "{self:?}") }
}

impl std::error::Error for Error {
}
