#![feature(collections)]
#![feature(cstr_to_str)]
#![feature(result_expect)]
#![feature(convert)]
#![feature(arc_unique)]
#![feature(iter_arith)]
#![feature(into_cow)]
#![feature(io)]
extern crate unicode_width;
extern crate term;
extern crate libc;
#[macro_use]
extern crate log;
extern crate env_logger;

use std::io::prelude::*;
use term::terminfo::TermInfo;

use std::collections::HashMap;
use std::io::BufReader;
use std::borrow::{Cow, IntoCow};
use std::error::Error;
use std::borrow::Borrow;
use std::fmt::{Display, Formatter};
use std::sync::{Arc, Mutex};
use std::sync::mpsc::{Sender, Receiver};
use std::fs::File;
use std::iter::FromIterator;

use std::sync::mpsc;
use std::fmt;
use std::thread;
use std::mem;
use std::env;

use ui::UI;
use terminal::Terminal;
use search::{SearchBase, LineInfo};

use types::*;
use constants::*;

mod constants;
#[macro_use]
mod error;
mod search;
mod types;
mod bis_c;
mod terminal;
mod ui;

fn main() {
    // init logging
    env_logger::init().expect("Failed to initialize logging");

    debug!("Getting terminal instance");

    let mut terminal = Terminal::create().expect("Failed to create terminal instance");

    debug!("Creating UI instance");

    let ui = UI::create().expect("Failed to create UI instance");

    let (emit, events) = mpsc::channel();
    let mut query = String::default();
    let mut search = None;
    let mut success = false;
    let mut best_match = None;

    {
        let emit = emit.clone();

        // start reading history
        thread::spawn(move || {
            let history_path = env::var("HISTFILE").expect("Failed to get bash history file");
            let input_file = BufReader::new(File::open(history_path).expect("Cauld not open history file"));
            let mut count = -1;
            let base = SearchBase::from_iter(input_file.lines().map(|maybe| {
                count += 1;
                LineInfo::new(maybe.expect("Failed to read line from file"), count)
            }));
            emit.send(Event::SearchReady(base)).and_then(|_| {
                debug!("Finished reading history");
                Ok(())
            });
        });
    }

    loop {
        match events.recv() {
            Err(_) => break,
            Ok(event) => {
                debug!("Got event: {:?}", event);
                match event {
                    Event::SearchReady(base) => {
                        search = Some(Arc::new(base));
                    },
                    Event::Input(chr) => {
                        query = match ui.input_char(query, chr) {
                            Err(s) => {
                                success = s;
                                // quit out of the event loop
                                break;
                            },
                            Ok((q, out)) => {
                                terminal.output_str(out).expect("Failed to write to output");
                                q
                            }
                        };

                        // don't search until we have a search base
                        search.clone().and_then(|base| {
                            let query = query.clone().into_cow();
                            let emit = emit.clone();

                            Some(thread::spawn(move || {
                                let result = base.query(query.clone());
                                emit.send(Event::Match(result, query)).and_then(|_| {
                                    trace!("Finished query");
                                    Ok(())
                                });
                            }))
                        });
                    },
                    Event::Match(matches, q) => {
                        if q == query {
                            best_match = matches.first().cloned();
                            // only draw matches for the current query
                            terminal.output_str(ui.render_matches(matches)).expect("Failed to draw matches");
                        }
                    }
                }

                debug!("Flushing output");
                terminal.flush().expect("Failed to flush terminal");
            }
        }
    }

    match best_match {
        Some(ref m) => {
            terminal.output_str(ui.render_best_match(m))
                .expect("Failed to draw best match");
        },
        None => {}
    }

    debug!("Flushing output");
    terminal.flush().expect("Failed to flush terminal");

    if success {
        match best_match {
            Some(ref m) => {
                terminal.insert_input(m)
                    .expect("Failed to insert input");
            },
            None => {}
        }
    }

    // restore the terminal
    mem::drop(terminal);
}
