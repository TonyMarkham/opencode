use std::fmt::{Display, Formatter, Result as FormatResult};
use std::panic::Location as PanicLocation;

#[cfg(feature = "serde")]
use serde::Serialize;

#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "serde", derive(Serialize))]
pub struct ErrorLocation {
    pub file: &'static str,
    pub line: u32,
    pub column: u32,
}

impl ErrorLocation {
    pub const fn from(location: &'static PanicLocation<'static>) -> Self {
        Self {
            file: location.file(),
            line: location.line(),
            column: location.column(),
        }
    }
}

impl Display for ErrorLocation {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> FormatResult {
        write!(formatter, "[{}:{}:{}]", self.file, self.line, self.column)
    }
}
