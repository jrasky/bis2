use std::io::prelude::*;

use std::io::{Stdin, Stdout, Chars};

use std::io;

use error::*;
use bis_c::*;

pub struct Terminal {
    input: Chars<Stdin>,
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
        let input = io::stdin().chars();

        match prepare_terminal() {
            Ok(_) => Ok(Terminal {
                input: input,
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

    pub fn input_char(&mut self) -> StrResult<Option<char>> {
        match self.input.next() {
            None => Ok(None),
            Some(Ok(chr)) => Ok(Some(chr)),
            Some(Err(err)) => errs!(err, "Failed to read character from input")
        }
    }

    pub fn flush(&mut self) -> StrResult<()> {
        self.output.flush().or_else(|err| {errs!(err, "Failed to flush output")})
    }

    pub fn insert_input<T: AsRef<str>>(&mut self, input: T) -> StrResult<()> {
        insert_input(input.as_ref()).or_else(|err| {errs!(err, "Failed to insert input")})
    }
}
