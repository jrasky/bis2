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
use std::io::prelude::*;

use std::io::Stdout;

use std::io;

use error::*;
use bis_c::*;

pub struct Terminal {
    output: Stdout
}

impl Drop for Terminal {
    fn drop(&mut self) {
        restore_terminal().expect("Failed to restore terminal");
    }
}

impl Terminal {
    pub fn create() -> StrResult<Terminal> {
        let output = io::stdout();

        match prepare_terminal() {
            Ok(_) => Ok(Terminal {
                output: output
            }),
            Err(e) => errs!(e, "Failed to prepare terminal")
        }
    }

    pub fn output_str<T: AsRef<str>>(&mut self, s: T) -> StrResult<()> {
        match write!(self.output, "{}", s.as_ref()) {
            Ok(_) => Ok(()),
            Err(err) => errs!(err, "Failed to write str to output")
        }
    }

    pub fn flush(&mut self) -> StrResult<()> {
        self.output.flush().or_else(|err| {errs!(err, "Failed to flush output")})
    }

    pub fn insert_input<T: AsRef<str>>(&mut self, input: T) -> StrResult<()> {
        insert_input(input.as_ref()).or_else(|err| {errs!(err, "Failed to insert input")})
    }
}
