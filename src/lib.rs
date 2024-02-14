use std::fmt::{Display, Formatter};

pub mod json_walker;
mod parser_core;
mod readers;
mod deserializer;

const NIL: u8 = 0;
const ROOT: char = '#';

//region error
#[derive(Debug, PartialEq)]
pub struct Error {
    kind: ErrorKind,
    msg: String,
}

impl Error {
    pub fn new_eos() -> Self {
        Error { kind: ErrorKind::EOS, msg: "End of stream".to_string() }
    }
}

#[derive(Debug, PartialEq)]
pub enum ErrorKind {
    EOS,
    Serde,
    ParseBoolError,
    ParseIntError,
    ParseFloatError,
    WrongDataType,
    OOPS,
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(&format!("Deserialization error: {:?}", self))
    }
}
//endregion
