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

use std::cmp;

use constants::*;

#[derive(PartialEq, Clone, Debug)]
pub enum TermStack {
    // here for correctness
    #[allow(dead_code)]
    Str(String),
    Int(isize),
    // here for correctness
    #[allow(dead_code)]
    Bool(bool),
}

pub struct Line {
    line: Arc<String>,
}

pub struct Escape {
    strings: HashMap<String, String>,
}

pub struct Matches {
    matches: Vec<Line>,
}

impl FromIterator<Arc<String>> for Matches {
    fn from_iter<T>(matches: T) -> Matches
        where T: IntoIterator<Item = Arc<String>>
    {
        Matches { matches: matches.into_iter().map(|item| Line::new(item)).collect() }
    }
}

impl Matches {
    pub fn len(&self) -> usize {
        self.matches.len()
    }

    pub fn get(&self, selected: usize) -> Option<&Line> {
        self.matches.get(selected)
    }

    pub fn render(&self, width: usize, selected: usize) -> String {
        let mut result = format!("");

        for (i, line) in self.matches.iter().enumerate() {
            write!(result, "{}", line.render(Some(width), i == selected)).unwrap();
        }

        result
    }
}

impl Line {
    pub fn new(line: Arc<String>) -> Line {
        Line { line: line }
    }

    pub fn get(&self) -> &String {
        self.line.borrow()
    }

    pub fn render(&self, width: Option<usize>, selected: bool) -> String {
        let mut result;

        if selected {
            result = format!("{}{}{}", MATCH_PRE, MATCH_SELECT, self.line);
        } else {
            result = format!("{}{}", MATCH_PRE, self.line);
        }

        width.map(|size| {
            while UnicodeWidthStr::width(result.as_str()) > size {
                result.pop();
            }
        });

        result
    }
}

impl Escape {
    pub fn create() -> Escape {
        let info = TermInfo::from_env().expect("Failed to get terminfo");
        let mut strings = HashMap::default();

        for (name, value) in info.strings.into_iter() {
            trace!("Inserting string {}", name);
            strings.insert(String::from(name), String::from_utf8(value).expect("String was not utf-8"));
        }

        Escape { strings: strings }
    }

    fn cursor_up(&self, by: usize) -> String {
        self.get_string("cuu", vec![TermStack::Int(by as isize)])
            .unwrap_or(format!(""))
    }

    pub fn restore_cursor(&self) -> String {
        self.get_string("rc", vec![]).unwrap_or(format!(""))
    }

    pub fn save_cursor(&self) -> String {
        self.get_string("sc", vec![]).unwrap_or(format!(""))
    }

    pub fn clear_screen(&self) -> String {
        self.get_string("ed", vec![]).unwrap_or(format!(""))
    }

    pub fn make_space(&self, rows: usize) -> String {
        let number = cmp::min(MATCH_NUMBER, rows - 1);
        format!("{}{}",
                String::from_iter(vec!['\n'; number as usize].into_iter()),
                self.cursor_up(number))
    }

    pub fn move_back(&self, by: usize) -> String {
        format!("{}{}{}",
                self.get_string("cub", vec![TermStack::Int(by as isize)])
                    .unwrap_or(format!("")),
                self.save_cursor(),
                self.clear_screen())
    }

    pub fn query_output(&self, chr: char) -> String {
        format!("{}{}{}", chr, self.save_cursor(), self.clear_screen())
    }

    pub fn matches_output(&self, matches: &Matches, width: usize, selected: usize) -> String {
        format!("{}{}{}",
                self.clear_screen(),
                matches.render(width, selected),
                self.restore_cursor())
    }

    pub fn match_down(&self, matches: &Matches, width: usize, selected: usize) -> String {
        // this gets us to the first match
        let mut result = format!("");

        // move down to the line before the last selection
        write!(result, "{}", String::from_iter(vec!['\n'; selected - 1].into_iter())).unwrap();

        // render the last line as non-selected
        match matches.get(selected - 1) {
            None => {
                // no such match, do nothing
                debug!("No such match: {}", selected - 1);
                return format!("");
            }
            Some(line) => {
                write!(result, "{}{}",
                    line.render(Some(width), false),
                    self.get_string("el", vec![]).unwrap_or(format!(""))).unwrap();
            }
        }

        // render the next line as selected
        match matches.get(selected) {
            None => {
                // no such match, do nothing
                debug!("No such match: {}", selected);
                return format!("");
            }
            Some(line) => {
                write!(result, "{}{}",
                    line.render(Some(width), true),
                    self.get_string("el", vec![]).unwrap_or(format!(""))).unwrap();
            }
        }

        // restore the cursor
        write!(result, "{}", self.restore_cursor()).unwrap();

        // return the result
        result
    }

    pub fn match_up(&self, matches: &Matches, width: usize, selected: usize) -> String {
        // this gets us to the first match
        let mut result = format!("");

        // move down to the line before the last selection
        write!(result, "{}",
            String::from_iter(vec!['\n'; selected].into_iter())).unwrap();

        // render the last line as selected
        match matches.get(selected) {
            None => {
                // no such match, do nothing
                debug!("No such match: {}", selected);
                return format!("");
            }
            Some(line) => {
                write!(result, "{}{}",
                    line.render(Some(width), true),
                    self.get_string("el", vec![]).unwrap_or(format!(""))).unwrap();
            }
        }

        // render the next line as selected
        match matches.get(selected + 1) {
            None => {
                // no such match, do nothing
                debug!("No such match: {}", selected + 1);
                return format!("");
            }
            Some(line) => {
                write!(result, "{}{}",
                    line.render(Some(width), false),
                    self.get_string("el", vec![]).unwrap_or(format!(""))).unwrap();
            }
        }

        // restore the cursor
        write!(result, "{}", self.restore_cursor()).unwrap();

        // return the result
        result
    }

    pub fn best_match_output(&self, matches: &Matches, selected: usize, recent: bool) -> String {
        matches.get(selected).map_or(format!("\n{}", self.clear_screen()), |line| {
            if recent {
                format!("recent{}{}\n{}", FINISH, line.get(), self.clear_screen())
            } else {
                format!("{}{}\n{}", FINISH, line.get(), self.clear_screen())
            }
        })
    }

    pub fn render_prompt(&self, rows: usize) -> String {
        format!("{}{}{}{}", self.make_space(rows), PROMPT, self.save_cursor(), self.clear_screen())
    }

    pub fn bell(&self) -> String {
        format!("{}", BEL)
    }

    fn get_string<T: Borrow<str>>(&self, name: T, params: Vec<TermStack>) -> Option<String> {
        // only implement what we're actually using in the UI
        let sequence = match self.strings.get(name.borrow()) {
            None => {
                trace!("No match for string: {:?}", name.borrow());
                return None;
            }
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
                        }
                        Some(o) => {
                            error!("Numeric print on non-numeric type: {:?}", o);
                        }
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
                                    Some(item) => stack.push(item.clone()),
                                    None => {
                                        error!("There was no parameter {}", idx);
                                    }
                                }
                            } else {
                                error!("Tried to print 0th paramater");
                            }
                        }
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
}
