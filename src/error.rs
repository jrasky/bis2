use std::fmt::{Display, Debug, Formatter};
use std::error::Error;
use std::borrow::Borrow;

use std::fmt;

macro_rules! errs {
    ($expr:expr, $($arg: tt)*) => ({
        $crate::std::result::Result::Err(
            $crate::error::StrError::new(format!($($arg)*),
                                         $crate::std::option::Option::Some(
                                             $crate::std::boxed::Box::new($expr))))
    })
}

pub type StrResult<T> = Result<T, StrError>;

pub struct StrError {
    description: String,
    cause: Option<Box<Error>>
}

impl Display for StrError {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        try!(write!(f, "{}", self.description()));
        match self.cause() {
            None => Ok(()),
            Some(error) => {
                write!(f, ": {}", error)
            }
        }
    }
}

impl Debug for StrError {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        Display::fmt(self, f)
    }
}

impl Error for StrError {
    fn description(&self) -> &str {
        self.description.as_ref()
    }

    fn cause(&self) -> Option<&Error> {
        match self.cause {
            None => None,
            Some(ref error) => Some(error.borrow())
        }
    }
}

impl StrError {
    pub fn new<T: Into<String>>(description: T, cause: Option<Box<Error>>) -> StrError {
        StrError {
            description: description.into(),
            cause: cause
        }
    }
}
