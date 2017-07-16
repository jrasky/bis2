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

use std::io::{Stdout, Stderr};

use std::io;

use bis_c::*;

pub struct Terminal {
    output: Stdout,
    error: Stderr,
    rows: u16,
    cols: u16,
}

impl Drop for Terminal {
    fn drop(&mut self) {
        restore_terminal();
    }
}

impl Terminal {
    pub fn create() -> Terminal {
        let output = io::stdout();

        let error = io::stderr();

        prepare_terminal();

        let (rows, cols) = get_terminal_size();

        Terminal {
            output,
            error,
            rows,
            cols,
        }
    }

    pub fn rows(&self) -> u16 {
        self.rows
    }

    pub fn cols(&self) -> u16 {
        self.cols
    }

    pub fn output_str<T: AsRef<str>>(&mut self, s: T) {
        write!(self.output, "{}", s.as_ref()).expect("Failed to write output");
    }

    pub fn flush(&mut self) {
        self.output.flush().expect("Failed to flush output");
    }

    pub fn insert_input<T: AsRef<str>>(&mut self, input: T) {
        write!(self.error, "{}", input.as_ref()).expect("Failed to write to stderr")
    }
}
