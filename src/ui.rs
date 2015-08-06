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
use unicode_width::*;
use term::terminfo::TermInfo;

use std::collections::HashMap;
use std::borrow::Borrow;
use std::fmt::Write;
use std::iter::FromIterator;
use std::sync::Arc;
use std::sync::mpsc::Sender;

use bis_c::*;
use constants::*;
use error::*;
use types::*;

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
    strings: HashMap<String, String>,
    control: HashMap<String, Option<String>>
}

impl UI {
    pub fn create() -> StrResult<UI> {
        let info = match TermInfo::from_env() {
            Ok(info) => info,
            Err(e) => return errs!(e, "Failed to get TermInfo")
        };

        let mut strings = HashMap::default();
        let mut control = HashMap::default();

        for (name, value) in info.strings.into_iter() {
            let strvalue = match String::from_utf8(value) {
                Ok(s) => s,
                Err(e) => return errs!(e, "failed to convert value to String")
            };

            // we only care about cuu1 and cud1
            if strvalue == "cuu1" {
                for i in 1..strvalue.len() - 1 {
                    control.entry(strvalue[0..i].to_owned()).or_insert(None);
                }
                
                control.insert(strvalue.clone(), Some(name.clone()));
            }

            strings.insert(name, strvalue);
        }

        let (rows, cols) = match get_terminal_size() {
            Ok(size) => size,
            Err(e) => return errs!(e, "Failed to get terminal size")
        };

        Ok(UI {
            rows: rows,
            cols: cols,
            strings: strings,
            control: control
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
    
    pub fn render_matches(&self, matches: Vec<Arc<String>>, number: usize) -> String {
        let mut result = format!("");

        for (idx, item) in matches.into_iter().enumerate() {
            // write the pre
            if idx == number {
                write!(result, "{}{}",
                       MATCH_PRE, MATCH_SELECT).expect("Failed to write pre to result");
            } else {
                write!(result, "{}",
                       MATCH_PRE).expect("Failed to write pre to result");
            }

            if UnicodeWidthStr::width(item.as_str()) > self.cols as usize {
                let mut owned = (*item).clone();
                while UnicodeWidthStr::width(owned.as_str()) > self.cols as usize {
                    // truncade long lines
                    owned.pop();
                }

                // draw the item
                write!(result, "{}", owned).expect("Writes to strings should not fail");
            } else {
                // draw the item
                write!(result, "{}", item).expect("Writes to strings should not fail");
            }
        }

        // restore the cursor
        write!(result, "{}", self.get_string(format!("rc"), vec![]).unwrap_or(format!("")))
            .expect("Writes to strings should not fail");

        result
    }

    pub fn render_best_match<T: AsRef<str>>(&self, query: T) -> String {
        format!("{}{}\n{}", FINISH, query.as_ref(),
                self.get_string(format!("clr_eos"), vec![]).unwrap_or(format!("")))
    }

    pub fn render_prompt(&self) -> String {
        format!("{}{}{}{}", String::from_iter(vec!['\n'; MATCH_NUMBER].into_iter()),
                self.get_string(format!("cuu"), vec![TermStack::Int(MATCH_NUMBER as isize)]).unwrap_or(format!("")),
                PROMPT,
                self.get_string(format!("sc"), vec![]).unwrap_or(format!("")))
    }

    pub fn input_char<T: AsRef<str>>(&self, emit: Sender<Event>, query: T, chr: char) -> String {
        if chr.is_control() {
            match chr {
                EOT => {
                    // stop
                    emit.send(Event::Quit(false)).expect("Failed to send quit signal");
                    format!("")
                },
                CTRL_U => {
                    // create our output
                    let output = format!("{}{}{}",
                                         self.get_string(format!("cub"),
                                                         vec![TermStack::Int(query.as_ref().len() as isize)])
                                         .unwrap_or(format!("")),
                                         self.get_string(format!("sc"), vec![]).unwrap_or(format!("")),
                                         self.get_string(format!("clr_eos"), vec![]).unwrap_or(format!("")));

                    emit.send(Event::Query(format!(""))).expect("Failed to send query signal");
                    output
                },
                '\n' => {
                    // exit
                    emit.send(Event::Quit(true)).expect("Failed to send quit signal");
                    format!("")
                },
                _ => {
                    // unknown character
                    // \u{7} is BEL
                    format!("\u{7}")
                }
            }
        } else if UnicodeWidthStr::width(query.as_ref()) + UnicodeWidthStr::width(PROMPT) +
            UnicodeWidthChar::width(chr).unwrap_or(0) >= self.cols as usize
        {
            // don't allow users to type past the end of one line
            format!("\u{7}")
        } else {
            // output the character and clear the screen
            emit.send(Event::Query(format!("{}{}", query.as_ref(), chr))).expect("Failed to send query signal");

            format!("{}{}{}", chr,
                    self.get_string(format!("sc"), vec![]).unwrap_or(format!("")),
                    self.get_string(format!("clr_eos"), vec![]).unwrap_or(format!("")))
        }
    }
}
