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
use std::sync::Arc;
use std::sync::mpsc::Sender;
use std::sync::atomic::{AtomicBool, Ordering};
use std::fs::File;
use std::iter::FromIterator;
use std::borrow::Borrow;
use std::thread::JoinHandle;
use std::collections::HashMap;

use std::env;
use std::io;
use std::thread;

use flx::{SearchBase, LineInfo};
use error::{StrError, StrResult};
use bis_c;

use types::*;
use constants::*;

pub fn start_threads(emit: Sender<Event>)
                     -> StrResult<(JoinHandle<()>, Arc<AtomicBool>)> {
    // mask sigint
    trys!(bis_c::mask_sigint(), "Failed to mask SIGINT");

    // start the sigint thread
    let signal_emit = emit.clone();
    thread::spawn(move || {
        read_signals(signal_emit);
    });

    // start reading history
    let history_emit = emit.clone();
    thread::spawn(move || {
        read_history(history_emit);
    });

    // start the input thread
    let input_stop = Arc::new(AtomicBool::new(false));
    let stop = input_stop.clone();
    let input_thread = thread::spawn(|| {
        read_input(emit, stop)
    });

    Ok((input_thread, input_stop))
}

fn read_history(emit: Sender<Event>) {
    let history_path = trysp!(env::var("HISTFILE"), "Failed to get bash history file");
    let input_file = BufReader::new(trysp!(File::open(history_path), "Cauld not open history file"));
    let mut count = -1.0;
    let base = SearchBase::from_iter(input_file.lines().map(|maybe| {
        count += 1.0;
        trace!("New line with count {}", count);
        LineInfo::new(maybe.expect("Failed to read line from file"), count)
    }));
    // if this fails, we can't search anything
    trysp!(emit.send(Event::SearchReady(base)), "Failed to emit search ready signal");
}

fn read_input(emit: Sender<Event>, stop: Arc<AtomicBool>) {
    // this thread is joined on quit, so none of its sends should fail
    let mut escape = None;

    // escape sequence tree, assume ANSI
    let mut control: HashMap<String, Option<Event>> = HashMap::default();
    control.insert(format!("["), None);
    control.insert(format!("[A"), Some(Event::KeyUp));
    control.insert(format!("[B"), Some(Event::KeyDown));

    // read characters
    for maybe_chr in io::stdin().chars() {
        match maybe_chr {
            Err(_) => {
                error!("Failed to read input, quitting");
                trysp!(emit.send(Event::Quit(false)), "Failed to emit quit signal");
                break;
            },
            Ok(ESC) => {
                escape = Some(String::default());
                trace!("Begin escape sequence");
            }
            Ok(chr) => {
                match escape {
                    None => {
                        if chr.is_control() {
                            match chr {
                                EOT => {
                                    // stop
                                    trysp!(emit.send(Event::Quit(false)),
                                           "Failed to send quit event");
                                },
                                CTRL_U => {
                                    // clear query
                                    trysp!(emit.send(Event::Clear),
                                           "Failed to send clear event");
                                },
                                '\n' => {
                                    // exit
                                    trysp!(emit.send(Event::Quit(true)),
                                           "Failed to send quit signal");
                                },
                                ESC => {
                                    // escape sequence
                                    escape = Some(format!(""));
                                }
                                _ => {
                                    // unknown control character
                                    trysp!(emit.send(Event::Bell),
                                           "Failed to send bell event");
                                }
                            }
                        } else {
                            trysp!(emit.send(Event::Input(chr)),
                                   "Failed to send character");
                        }
                    },
                    Some(mut seq) => {
                        seq.push(chr);
                        match control.get(&seq) {
                            None => {
                                // no possible escape sequence like this
                                trysp!(emit.send(Event::Bell),
                                       "Failed to send bell event");
                                trysp!(emit.send(Event::Input(chr)),
                                       "Failed to send character");
                                escape = None;
                            },
                            Some(&None) => {
                                // keep going
                                escape = Some(seq);
                            },
                            Some(&Some(ref event)) => {
                                // send the appropriate event
                                let cloned = trysp!(
                                    event.maybe_clone()
                                        .ok_or(StrError::new(
                                            "Event {:?} could not be cloned", None)),
                                    "Failed to create event");
                                trysp!(emit.send(cloned),
                                       "Failed to send escape event");
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

fn read_signals(emit: Sender<Event>) {
    // wait for a sigint
    trysp!(bis_c::wait_sigint(), "Failed to wait for sigint");

    debug!("Caught sigint");

    // send a quit signal
    let _ = emit.send(Event::Quit(false));
    // might happen after events is closed, so don't fail
}

pub fn start_query(emit: Sender<Event>, base: Arc<SearchBase>, query: Arc<String>) {
    let result = base.query::<&String>(query.borrow(), MATCH_NUMBER);
    let _ = emit.send(Event::Match(result, query)).and_then(|_| {
        trace!("Finished query");
        Ok(())
    });
    // don't panic on fail send, events might be already closed
}