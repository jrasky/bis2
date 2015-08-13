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

use std::cmp;

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

pub struct UI {
    rows: u16,
    cols: u16,
    strings: HashMap<String, String>,
    control: HashMap<String, Option<String>>,
    emit: Sender<Event>
}

impl UI {
    pub fn create(emit: Sender<Event>) -> StrResult<UI> {
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

            strings.insert(name, strvalue);
        }

        let (rows, cols) = match get_terminal_size() {
            Ok(size) => size,
            Err(e) => return errs!(e, "Failed to get terminal size")
        };

        // assume an ANSI terminal for input sequences
        control.insert(format!("["), None);
        control.insert(format!("[A"), Some(format!("cuu1")));
        control.insert(format!("[B"), Some(format!("cud1")));

        Ok(UI {
            rows: rows,
            cols: cols,
            strings: strings,
            control: control,
            emit: emit
        })
    }

    fn get_string<T: Borrow<String>>(&self, name: T, params: Vec<TermStack>) -> Option<String> {
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

    pub fn render_selection(&self, maybe_matches: Option<&[Arc<String>]>, number: usize, old: usize) -> String {
        if number == old {
            return format!("");
        }

        let matches;
        match maybe_matches {
            Some(m) => {
                matches = m;
            },
            None => {
                return format!("");
            }
        }

        let mut result = format!("\n");

        if old > 0 {
            trysp!(write!(result, "{}",
                          self.get_string(format!("cud"),
                                          vec![TermStack::Int(old as isize)]).unwrap_or(format!(""))),
                   "Failed to write to result");
        }

        trysp!(write!(result, "{}{}",
                      self.get_string(format!("clr_eol"), vec![]).unwrap_or(format!("")),
                      matches[old]),
               "Failed to write to result");

        if number > old + 1 {
            trysp!(write!(result, "{}",
                          self.get_string(format!("cud"), 
                                          vec![TermStack::Int((number - old - 1) as isize)])
                          .unwrap_or(format!(""))),
                   "Foiled to write to result");
        } else if old > number {
            trysp!(write!(result, "{}",
                          self.get_string(format!("cuu"), 
                                          vec![TermStack::Int((old - number + 1) as isize)])
                          .unwrap_or(format!(""))),
                   "Failed to write to result");
        } else {
            // do nothing
        }

        format!("{}\n{}{}{}{}", result,
                self.get_string(format!("clr_eol"), vec![]).unwrap_or(format!("")),
                MATCH_SELECT, self.truncate_string(matches[number].clone()),
                self.get_string(format!("rc"), vec![]).unwrap_or(format!("")))
    }

    fn truncate_string(&self, item: Arc<String>) -> Arc<String> {
        if UnicodeWidthStr::width(item.as_str()) > self.cols as usize {
            let mut owned = (*item).clone();
            while UnicodeWidthStr::width(owned.as_str()) > self.cols as usize {
                owned.pop();
            }
            Arc::new(owned)
        } else {
            item
        }
    }
    
    pub fn render_matches(&self, maybe_matches: Option<&[Arc<String>]>, number: usize) -> String {
        let matches;
        match maybe_matches {
            Some(m) => {
                matches = m;
            },
            None => {
                return format!("");
            }
        }

        let mut result = format!("");
        let match_number = cmp::min(MATCH_NUMBER, self.rows as usize - 1);

        for (idx, item) in matches.into_iter().enumerate() {
            if idx >= match_number {
                // don't render past the end of the screen
                break;
            }

            // write the pre
            if idx == number {
                write!(result, "{}{}",
                       MATCH_PRE, MATCH_SELECT).expect("Failed to write pre to result");
            } else {
                write!(result, "{}",
                       MATCH_PRE).expect("Failed to write pre to result");
            }

            // draw the item
            write!(result, "{}", self.truncate_string(item.clone())).expect("Writes to strings should not fail");
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
        let number = cmp::min(MATCH_NUMBER, self.rows as usize - 1);
        format!("{}{}{}{}", String::from_iter(vec!['\n'; number].into_iter()),
                self.get_string(format!("cuu"), vec![TermStack::Int(number as isize)]).unwrap_or(format!("")),
                PROMPT,
                self.get_string(format!("sc"), vec![]).unwrap_or(format!("")))
    }

    fn input_query<T: AsRef<str>>(&self, query: T, chr: char) -> StrResult<(String, Option<String>)> {
        if chr.is_control() {
            match chr {
                ESC => {
                    // escape sequence
                    Ok((format!(""), Some(format!(""))))
                },
                EOT => {
                    // stop
                    trys!(self.emit.send(Event::Quit(false)), "Failed to send quit signal");
                    Ok((format!(""), None))
                },
                CTRL_U => {
                    // create our output
                    let output = format!("{}{}{}",
                                         self.get_string(format!("cub"),
                                                         vec![TermStack::Int(query.as_ref().len() as isize)])
                                         .unwrap_or(format!("")),
                                         self.get_string(format!("sc"), vec![]).unwrap_or(format!("")),
                                         self.get_string(format!("clr_eos"), vec![]).unwrap_or(format!("")));

                    trys!(self.emit.send(Event::Query(format!(""))), "Failed to send query signal");
                    Ok((output, None))
                },
                '\n' => {
                    // exit
                    trys!(self.emit.send(Event::Quit(true)), "Failed to send quit signal");
                    Ok((format!(""), None))
                },
                _ => {
                    // unknown character
                    // \u{7} is BEL
                    Ok((format!("\u{7}"), None))
                }
            }
        } else if UnicodeWidthStr::width(query.as_ref()) + UnicodeWidthStr::width(PROMPT) +
            UnicodeWidthChar::width(chr).unwrap_or(0) >= self.cols as usize
        {
            // don't allow users to type past the end of one line
            Ok((format!("\u{7}"), None))
        } else {
            // output the character and clear the screen
            trys!(self.emit.send(Event::Query(format!("{}{}", query.as_ref(), chr))), "Failed to send query signal");

            Ok((format!("{}{}{}", chr,
                        self.get_string(format!("sc"), vec![]).unwrap_or(format!("")),
                        self.get_string(format!("clr_eos"), vec![]).unwrap_or(format!(""))),
                None))
        }
    }

    fn input_escape<T: AsRef<str>, V: AsRef<str>>(&self, query: T, escape: V, chr: char) -> StrResult<(String, Option<String>)> {
        let esc_query = format!("{}{}", escape.as_ref(), chr);
        debug!("Escape query: {}", esc_query);
        match self.control.get(&esc_query) {
            None => {
                // no possible escape sequence like this
                trys!(self.emit.send(Event::Query(format!("{}{}", query.as_ref(), chr))), "Failed to send query signal");

                // BEL and then print the character
                Ok((format!("{}{}{}{}", BEL, chr,
                            self.get_string(format!("sc"), vec![]).unwrap_or(format!("")),
                            self.get_string(format!("clr_eos"), vec![]).unwrap_or(format!(""))),
                    None))
            },
            Some(&None) => {
                // keep going
                Ok((format!(""), Some(format!("{}{}", escape.as_ref(), chr))))
            },
            Some(&Some(ref name)) => {
                if name == "cuu1" {
                    trys!(self.emit.send(Event::MatchUp), "Failed to send match up signal");
                    Ok((format!(""), None))
                } else if name == "cud1" {
                    trys!(self.emit.send(Event::MatchDown), "Failed to send match down signal");
                    Ok((format!(""), None))
                } else {
                    Err(StrError::new(format!("Unknown escape sequence: {:?}", name), None))
                }
            }
        }
    }

    pub fn input_chr<T: AsRef<str>, V: AsRef<str>>(&self, query: T, escape: Option<V>,
                                                   chr: char) -> StrResult<(String, Option<String>)> {
        match escape {
            Some(esc) => self.input_escape(query, esc, chr),
            None => self.input_query(query, chr)
        }
    }

    pub fn match_down(&self, number: usize, total: Option<usize>) -> StrResult<String> {
        match total {
            Some(total) => {
                if number + 1 >= total {
                    Ok(format!("{}", BEL))
                } else {
                    trys!(self.emit.send(Event::Select(number + 1)), "Failed to send select event");
                    Ok(format!(""))
                }
            },
            None => {
                debug!("Match down without any matches");
                Ok(format!("{}", BEL))
            }
        }
    }

    pub fn match_up(&self, number: usize) -> StrResult<String> {
        if number <= 0 {
            Ok(format!("{}", BEL))
        } else {
            trys!(self.emit.send(Event::Select(number - 1)), "Failed to send select event");
            Ok(format!(""))
        }
    }
}
