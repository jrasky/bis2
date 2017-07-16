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

use std::io::BufReader;
use std::sync::{Arc, MutexGuard};
use std::sync::mpsc::Sender;
use std::sync::atomic::{AtomicBool, Ordering};
use std::fs::File;
use std::thread::JoinHandle;
use std::collections::{HashMap, VecDeque};
use std::iter::FromIterator;

use std::env;
use std::io;
use std::thread;

use serde_json;

use flx::{SearchBase, LineInfo};

use types::*;
use constants::*;

pub fn start_threads(emit: Sender<Event>) -> (JoinHandle<()>, Arc<AtomicBool>) {
    // start reading completions
    let completions_emit = emit.clone();
    thread::spawn(move || {
        read_completions(completions_emit);
    });

    // start the input thread
    let input_stop = Arc::new(AtomicBool::new(false));
    let stop = input_stop.clone();
    let input_thread = thread::spawn(|| read_input(emit, stop));

    (input_thread, input_stop)
}

fn read_completions(emit: Sender<Event>) {
    let mut completions_path = env::home_dir().unwrap_or("".into());
    completions_path.push(".bis2_completions");

    trace!("Completions path: {:?}", completions_path);

    let try_completions_file = File::open(completions_path);
    if let Ok(completions_file) = try_completions_file {
        trace!("Reading completions");
        let completions = serde_json::from_reader(BufReader::new(completions_file))
            .expect("Failed to read completions file");

        trace!("Read completions");

        emit.send(Event::CompletionsReady(completions)).unwrap();
    } else {
        trace!("No completions found");
        emit.send(Event::CompletionsReady(Completions::new())).unwrap();
    }
}

pub fn read_history(completions: MutexGuard<Completions>, emit: Sender<Event>) {
    let history_path = match env::var("HISTFILE") {
        Ok(path) => path.into(),
        Err(env::VarError::NotPresent) => {
            debug!("History file not found, defaulting to ~/.bash_history");
            let mut home = env::home_dir().unwrap_or("".into());
            home.push(".bash_history");
            home
        },
        Err(e) => {
            panic!("Failed to get history file path: {}", e);
        }
    };

    trace!("History path: {:?}", history_path);

    // try to get the current path
    let current_path = env::current_dir().ok();

    trace!("Current path: {:?}", current_path);

    let input_file = BufReader::new(File::open(history_path).expect("Failed to open history file"));
    let mut count = 0.0;
    let mut set: HashMap<String, LineInfo> = HashMap::new();
    let mut short: VecDeque<String> = VecDeque::new();

    for maybe_line in input_file.lines() {
        if let Ok(line) = maybe_line {
            let item = set.entry(line.clone()).or_insert(LineInfo::new(line.as_str(), 0.0));
            let old_count = item.get_factor();

            let path_score = if let Some(ref path) = current_path {
                // count the path score each time we see this line
                completions.get_score(&line, path)
            } else {
                0.0
            };

            let maybe_index = short.iter().enumerate()
                .find(|&(_, ref item)| line == **item)
                .map(|(index, _)| index);

            if let Some(index) = maybe_index {
                short.remove(index);
            }

            short.push_back(line);

            while short.len() > 10 {
                short.pop_front();
            }

            item.set_factor(old_count + count + path_score);
            count += 1.0;
        }
    }

    // extract shortlist
    let mut recent = vec![];

    for item in short.into_iter().rev() {
        recent.push(item);
    }

    // send off recent history
    emit.send(Event::HistoryReady(recent)).unwrap();

    // create the search base
    let base = SearchBase::from_iter(set.into_iter().map(|(_, info)| info));

    // if this fails, we can't search anything
    emit.send(Event::SearchReady(base)).unwrap();
}

fn read_input(emit: Sender<Event>, stop: Arc<AtomicBool>) {
    // this thread is joined on quit, so none of its sends should fail
    let mut escape = None;

    // escape sequence tree, assume ANSI
    let mut control: HashMap<String, Option<Event>> = HashMap::default();
    control.insert(format!("["), None);
    control.insert(format!("[A"), Some(Event::KeyUp));
    control.insert(format!("[B"), Some(Event::KeyDown));

    let chars = Chars {
        inner: io::stdin()
    };

    // read characters
    for maybe_chr in chars {
        match maybe_chr {
            Err(_) => {
                error!("Failed to read input, quitting");
                emit.send(Event::Quit(false)).unwrap();
                break;
            }
            Ok(ESC) => {
                escape = Some(String::default());
                trace!("Begin escape sequence");
            }
            Ok(chr) => {
                match escape {
                    None => {
                        if chr.is_control() {
                            if chr == EOT || chr == CTRL_C {
                                // stop
                                trace!("Got EOT or CTRL_C");
                                emit.send(Event::Quit(false)).unwrap();
                                break;
                            } else if chr == CTRL_U {
                                // clear query
                                trace!("Got CTRL_U");
                                emit.send(Event::Clear).unwrap();
                            } else if chr == CTRL_R {
                                // key up
                                trace!("Got CTRL_R");
                                emit.send(Event::KeyDown).unwrap();
                            } else if chr == CTRL_S {
                                // key down
                                trace!("Got CTRL_S");
                                emit.send(Event::KeyUp).unwrap();
                            } else if chr == '\n' || chr == CR {
                                // exit
                                trace!("Got newline");
                                emit.send(Event::Quit(true)).unwrap();
                                break;
                            } else if chr == ESC {
                                // escape sequence
                                trace!("Got ESC");
                                escape = Some(format!(""));
                            } else if chr == BSPC || chr == DEL {
                                // backspace
                                trace!("Got backspace");
                                emit.send(Event::Backspace).unwrap();
                            } else {
                                debug!("Unknown control character {:?}", chr);
                                emit.send(Event::Bell).unwrap();
                            }
                        } else {
                            emit.send(Event::Input(chr)).unwrap();
                        }
                    }
                    Some(mut seq) => {
                        seq.push(chr);
                        trace!("Sequence: {:?}", seq);
                        match control.get(&seq) {
                            None => {
                                // no possible escape sequence like this
                                emit.send(Event::Bell).unwrap();
                                emit.send(Event::Input(chr)).unwrap();
                                escape = None;
                            }
                            Some(&None) => {
                                // keep going
                                escape = Some(seq);
                            }
                            Some(&Some(ref event)) => {
                                // send the appropriate event
                                let cloned = event.maybe_clone().unwrap();
                                emit.send(cloned).unwrap();
                                escape = None;
                            }
                        }
                    }
                }
            }
        }

        // check for requested stop
        if stop.load(Ordering::Relaxed) {
            debug!("Input thread exiting");
            break;
        }
    }
}

pub fn start_query(emit: Sender<Event>, base: Arc<SearchBase>, query: String) {
    let result = base.query(&query, MATCH_NUMBER);
    let _ = emit.send(Event::Match(result.into_iter().map(|s| s.into()).collect(), query)).and_then(|_| {
        trace!("Finished query");
        Ok(())
    });
    // don't panic on fail send, events might be already closed
}

// Copied from the standard library so I can use it in stable
struct Chars<R> {
    inner: R
}

impl<R: io::Read> Iterator for Chars<R> {
    type Item = Result<char, ()>;

    fn next(&mut self) -> Option<Result<char, ()>> {
        let first_byte = match read_one_byte(&mut self.inner) {
            None => return None,
            Some(Ok(b)) => b,
            Some(Err(_)) => return Some(Err(()))
        };

        let width = utf8_char_width(first_byte);
        
        if width == 1 { return Some(Ok(first_byte as char)) }
        if width == 0 { return Some(Err(())) }

        let mut buf = [first_byte, 0, 0, 0];
        {
            let mut start = 1;
            while start < width {
                match self.inner.read(&mut buf[start..width]) {
                    Ok(0) => return Some(Err(())),
                    Ok(n) => start += n,
                    Err(ref e) if e.kind() == io::ErrorKind::Interrupted => continue,
                    Err(_) => return Some(Err(())),
                }
            }
        }

        Some(match ::std::str::from_utf8(&buf[..width]).ok() {
            Some(s) => Ok(s.chars().next().unwrap()),
            None => Err(())
        })
    }
}

fn read_one_byte(reader: &mut Read) -> Option<Result<u8, ()>> {
    let mut buf = [0];
    loop {
        return match reader.read(&mut buf) {
            Ok(0) => None,
            Ok(..) => Some(Ok(buf[0])),
            Err(ref e) if e.kind() == io::ErrorKind::Interrupted => continue,
            Err(_) => Some(Err(())),
        };
    }
}