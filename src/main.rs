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
#![feature(collections)]
#![feature(cstr_to_str)]
#![feature(result_expect)]
#![feature(convert)]
#![feature(iter_arith)]
#![feature(io)]
extern crate unicode_width;
extern crate term;
extern crate libc;
#[macro_use]
extern crate log;
extern crate env_logger;

use std::io::prelude::*;

use std::io::BufReader;
use std::sync::Arc;
use std::sync::mpsc::Sender;
use std::sync::atomic::{AtomicBool, Ordering};
use std::fs::File;
use std::iter::FromIterator;
use std::thread::JoinHandle;
use std::borrow::Borrow;

use std::sync::mpsc;
use std::thread;
use std::mem;
use std::env;
use std::io;

use ui::UI;
use terminal::Terminal;
use search::{SearchBase, LineInfo};

use types::*;

mod constants;
#[macro_use]
mod error;
mod search;
mod types;
mod bis_c;
mod terminal;
mod ui;

fn read_history(emit: Sender<Event>) {
    thread::spawn(move || {
        let history_path = env::var("HISTFILE").expect("Failed to get bash history file");
        let input_file = BufReader::new(File::open(history_path).expect("Cauld not open history file"));
        let mut count = -1;
        let base = SearchBase::from_iter(input_file.lines().map(|maybe| {
            count += 1;
            trace!("New line with count {}", count);
            LineInfo::new(maybe.expect("Failed to read line from file"), count)
        }));
        // if this fails, we can't search anything
        emit.send(Event::SearchReady(base)).expect("Failed to emit search ready signal");
    });
}

fn read_input(emit: Sender<Event>, stop: Arc<AtomicBool>) -> JoinHandle<()> {
    thread::spawn(move || {
        // this thread is joined on quit, so none of its sends should fail
        for maybe_chr in io::stdin().chars() {
            match maybe_chr {
                Err(_) => {
                    error!("Failed to read input, quitting");
                    emit.send(Event::Quit(false)).expect("Failed to emit quit signal");
                    break;
                },
                Ok(chr) => {
                    emit.send(Event::Input(chr)).expect("Failed to send character");
                    if stop.load(Ordering::Relaxed) {
                        debug!("Input thread exiting");
                        break;
                    }
                }
            }
        }
    })
}

fn read_signals(emit: Sender<Event>) {
    thread::spawn(move || {
        // wait for a sigint
        bis_c::wait_sigint().expect("Failed to wait for sigint");

        debug!("Caught sigint");

        // send a quit signal
        emit.send(Event::Quit(false)).is_ok();
        // might happen after events is closed, so don't fail
    });
}

fn start_query(emit: Sender<Event>, base: Arc<SearchBase>, query: Arc<String>) {
    thread::spawn(move || {
        let result = base.query::<&String>(query.borrow());
        emit.send(Event::Match(result, query)).and_then(|_| {
            trace!("Finished query");
            Ok(())
        }).is_ok();
        // don't panic on fail send, events might be already closed
    });
}

fn main() {
    // init logging
    env_logger::init().expect("Failed to initialize logging");

    debug!("Getting terminal instance");

    let mut terminal = Terminal::create().expect("Failed to create terminal instance");

    debug!("Creating UI instance");

    let ui = UI::create().expect("Failed to create UI instance");

    let (emit, events) = mpsc::channel();
    let mut query = Arc::new(String::default());
    let mut search = None;
    let mut success = false;
    let mut best_match = None;
    let input_stop = Arc::new(AtomicBool::new(false));
    let match_number = 0;

    // mask sigint
    bis_c::mask_sigint().expect("Failed to mask sigint");

    // wait for a sigint
    read_signals(emit.clone());

    // start reading history
    read_history(emit.clone());

    // start the input thread
    let input_thread = read_input(emit.clone(), input_stop.clone());

    // draw the prompt
    terminal.output_str(ui.render_prompt()).expect("Failed to render prompt");

    // flush the terminal
    terminal.flush().expect("Failed to flush terminal");

    loop {
        match events.recv() {
            Err(_) => break,
            Ok(event) => {
                match event {
                    Event::SearchReady(base) => {
                        search = Some(Arc::new(base));
                        // execute a query if it isn't empty
                        if !query.is_empty() {
                            search.clone().map(|base| {start_query(emit.clone(), base, query.clone())});
                        }
                    },
                    Event::Query(q) => {
                        query = Arc::new(q);
                        if !query.is_empty() {
                            search.clone().map(|base| {start_query(emit.clone(), base, query.clone())});
                        }
                    }
                    Event::Input(chr) => {
                        debug!("Got input event: {:?}", chr);
                        terminal.output_str(ui.input_char::<&String>(emit.clone(), query.borrow(), chr))
                            .expect("Failed to write output");
                    },
                    Event::Match(matches, q) => {
                        debug!("Got match event: {:?}, {:?}", matches, q);
                        if q == query {
                            best_match = matches.first().cloned();
                            // only draw matches for the current query
                            terminal.output_str(ui.render_matches(matches, match_number)).expect("Failed to draw matches");
                        }
                    },
                    Event::Quit(s) => {
                        debug!("Got quit event: {:?}", s);
                        success = s;
                        break;
                    }
                }

                debug!("Flushing output");
                terminal.flush().expect("Failed to flush terminal");
            }
        }
    }

    // prompt the input thread to stop
    input_stop.store(true, Ordering::Relaxed);

    // insert a bogus byte to wake it up
    terminal.insert_input(" ").expect("Failed to insert bogus byte");

    // join the thread
    input_thread.join().expect("Failed to wait for input thread");

    // draw the best match if it exists
    match best_match {
        Some(ref m) => {
            terminal.output_str(ui.render_best_match::<&String>(m.borrow()))
                .expect("Failed to draw best match");
        },
        None => {
            terminal.output_str("\n").expect("Failed to draw newline");
        }
    }

    debug!("Flushing output");
    terminal.flush().expect("Failed to flush terminal");

    if success {
        match best_match {
            Some(ref m) => {
                terminal.insert_input::<&String>(m.borrow())
                    .expect("Failed to insert input");
            },
            None => {}
        }
    }

    // restore the terminal
    mem::drop(terminal);
}
