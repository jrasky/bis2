// Copyright 2015 Jerome Rasky <jerome@rasky.co>
//
// Licensed under the Apache License, version 2.0 (the "License"); you may not
// use this file except in compliance with the License. You may obtain a copy of
// the License at
//
//     <http://www.apache.org/licenses/LICENSE-2.0>
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS, WITHOUT
// WARRANTIES OR CONDITIONS OF ANY KIND, either expressed or implied. See the
// License for the specific language concerning governing permissions and
// limitations under the License.
use std::fmt::{Display, Debug, Formatter};
use std::error::Error;
use std::borrow::Borrow;

use std::fmt;

macro_rules! errs {
    ($expr: expr, $($arg: tt)*) => ({
        $crate::std::result::Result::Err(
            $crate::error::StrError::new(format!($($arg)*),
                                         $crate::std::option::Option::Some(
                                             $crate::std::boxed::Box::new($expr))))
    })
}

macro_rules! trys {
    ($expr: expr, $($arg: tt)*) => (match expr {
        $crate::std::result::Result::Ok(v) => v,
        $crate::std::result::Result::Err(e) => return errs!(e, $($arg)*)
    })
}

pub type StrResult<T> = Result<T, StrError>;

#[derive(Debug)]
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
