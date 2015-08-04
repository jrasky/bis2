use unicode_width::*;
use term::terminfo::TermInfo;

use std::collections::HashMap;
use std::borrow::{Borrow, Cow, IntoCow};
use std::fmt::Write;

use bis_c::*;
use types::*;
use constants::*;
use error::*;

#[derive(PartialEq, Clone, Debug)]
pub enum TermStack {
    // here for correctness
    #[allow(dead_code)]
    Str(String),
    Int(isize),
    // here for correctness
    #[allow(dead_code)]
    Bool(bool)
}

#[derive(Debug)]
pub struct UI {
    rows: u16,
    cols: u16,
    strings: HashMap<String, String>
}

impl UI {
    pub fn create() -> StrResult<UI> {
        let info = match TermInfo::from_env() {
            Ok(info) => info,
            Err(e) => return errs!(e, "Failed to get TermInfo")
        };

        let mut strings = HashMap::default();

        for (name, value) in info.strings.into_iter() {
            strings.insert(name, match String::from_utf8(value) {
                Ok(s) => s,
                Err(e) => return errs!(e, "failed to convert value to String")
            });
        }

        let (rows, cols) = match get_terminal_size() {
            Ok(size) => size,
            Err(e) => return errs!(e, "Failed to get terminal size")
        };

        Ok(UI {
            rows: rows,
            cols: cols,
            strings: strings
        })
    }

    pub fn get_string<T: Borrow<String>>(&self, name: T, params: Vec<TermStack>) -> Option<String> {
        // only implement what we're actually using in the UI
        let sequence = match self.strings.get(name.borrow()) {
            None => {
                trace!("No match for string: {:?}", name.borrow());
                return None;
            },
            Some(s) => {
                trace!("Matched string: {:?}", s);
                s.clone()
            }
        };

        let mut escaped = false;
        let mut stack: Vec<TermStack> = vec![];
        let mut result = String::default();
        let mut escape = String::default();

        // only implement the sequences we care about

        for c in sequence.chars() {
            if !escaped {
                if c == '%' {
                    escaped = true;
                } else {
                    result.push(c);
                }
            } else if escape.is_empty() {
                if c == 'd' {
                    match stack.pop() {
                        Some(TermStack::Int(c)) => {
                            result.push_str(format!("{}", c).as_ref());
                        },
                        Some(o) => {
                            error!("Numeric print on non-numeric type: {:?}", o);
                        },
                        None => {
                            error!("Stack was empty on print");
                        }
                    }

                    escaped = false;
                } else if c == 'p' {
                    escape.push('p');
                } else {
                    error!("Unknown escape character: {:?}", c);
                    escaped = false;
                }
            } else {
                if escape == "p" {
                    match c.to_digit(10) {
                        Some(idx) => {
                            if idx != 0 {
                                match params.get(idx as usize - 1) {
                                    Some(item) => {
                                        stack.push(item.clone())
                                    },
                                    None => {
                                        error!("There was no parameter {}", idx);
                                    }
                                }
                            } else {
                                error!("Tried to print 0th paramater");
                            }
                        },
                        None => {
                            error!("Paramater number was not a digit");
                        }
                    }

                    escape.clear();
                    escaped = false;
                } else {
                    error!("Unknown escape sequence: {:?}", escape);
                    escape.clear();
                    escaped = false;
                }
            }
        }

        trace!("Returning result: {:?}", result);

        // return result
        Some(result)
    }
    
    pub fn render_matches(&self, matches: Vec<Cow<'static, str>>) -> String {
        let mut result = format!("");

        for item in matches.into_iter() {
            if UnicodeWidthStr::width(item.as_ref()) > self.cols as usize {
                let mut owned = item.into_owned();
                while UnicodeWidthStr::width(owned.as_str()) > self.cols as usize {
                    // truncade long lines
                    owned.pop();
                }

                // draw the item
                write!(result, "\n{}", owned).expect("Writes to strings should not fail");
            } else {
                // draw the item
                write!(result, "\n{}", item).expect("Writes to strings should not fail");
            }
        }

        // restore the cursor
        write!(result, "{}", self.get_string(format!("rc"), vec![]).unwrap_or(format!("")))
            .expect("Writes to strings should not fail");

        result
    }

    pub fn render_best_match<T: AsRef<str>>(&self, query: T) -> String {
        format!("{}{}{}\n", FINISH, query.as_ref(),
                self.get_string(format!("clr_eos"), vec![]).unwrap_or(format!("")))
    }

    pub fn input_char(&self, query: Cow<'static, str>, chr: char) -> Result<(Cow<'static, str>, String), bool> {
        if chr.is_control() {
            match chr {
                EOT => {
                    // stop
                    Err(false)
                },
                CTRL_U => {
                    // create our output
                    let output = format!("{}{}",
                                         self.get_string(format!("cub"),
                                                         vec![TermStack::Int(query.len() as isize)])
                                         .unwrap_or(format!("")),
                                         self.get_string(format!("clr_eos"), vec![]).unwrap_or(format!("")));

                    Ok(("".into_cow(), output))
                },
                '\n' => {
                    // exit
                    Err(true)
                },
                _ => {
                    // unknown character
                    // \u{7} is BEL
                    Ok((query, format!("\u{7}")))
                }
            }
        } else if UnicodeWidthStr::width(query.as_ref()) + UnicodeWidthStr::width(PROMPT) +
            UnicodeWidthChar::width(chr).unwrap_or(0) >= self.cols as usize
        {
            // don't allow users to type past the end of one line
            Ok(("".into_cow(), format!("\u{7}")))
        } else {
            // output the character and clear the screen
            let mut query = query.into_owned();
            query.push(chr);

            Ok((query.into_cow(),
                format!("{}{}{}", chr,
                        self.get_string(format!("sc"), vec![]).unwrap_or(format!("")),
                        self.get_string(format!("clr_eos"), vec![]).unwrap_or(format!(""))
                        )))
        }
    }
}
