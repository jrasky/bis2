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
    output: Stdout,
    rows: u16,
    cols: u16,
}

impl Drop for Terminal {
    fn drop(&mut self) {
        restore_terminal().expect("Failed to restore terminal");

        // signal masking is per-thread, so we don't need to unmask it on exit
    }
}

impl Terminal {
    pub fn create() -> StrResult<Terminal> {
        let output = io::stdout();

        trys!(prepare_terminal(), "Failed to prepare terminal");

        trys!(mask_sigint(), "Failed to mask sigint");

        let (rows, cols) = trys!(get_terminal_size(), "Failed to get terminal size");

        Ok(Terminal {
            output: output,
            rows: rows,
            cols: cols,
        })
    }

    pub fn rows(&self) -> u16 {
        self.rows
    }

    pub fn cols(&self) -> u16 {
        self.cols
    }

    pub fn output_str<T: AsRef<str>>(&mut self, s: T) -> StrResult<()> {
        Ok(trys!(write!(self.output, "{}", s.as_ref()),
                 "Failed to write str to output"))
    }

    pub fn flush(&mut self) -> StrResult<()> {
        Ok(trys!(self.output.flush(), "Failed to flush output"))
    }

    pub fn insert_input<T: AsRef<str>>(&mut self, input: T) -> StrResult<()> {
        Ok(trys!(insert_input(input.as_ref()), "Failed to insert input"))
    }
}
