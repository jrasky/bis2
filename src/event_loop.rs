
use std::io::prelude::*;

use std::sync::Arc;
use std::sync::mpsc::{Sender, Receiver};
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::JoinHandle;
use std::borrow::Borrow;
use std::iter::FromIterator;
use std::error::Error;

use std::sync::mpsc;
use std::thread;
use std::cmp;
use std::raw;
use std::mem;

use terminal::Terminal;
use search::SearchBase;
use threads;

use ui::*;
use error::*;
use types::*;

pub struct EventLoop {
    emit: Sender<Event>,
    events: Receiver<Event>,
    terminal: Terminal,
    escape: Escape,
    matches: Matches,
    selected: usize,
    query: Arc<String>,
    success: bool,
    input_thread: Option<JoinHandle<()>>,
    input_stop: Arc<AtomicBool>,
    search: Option<Arc<SearchBase>>
}

impl EventLoop {
    pub fn create() -> StrResult<EventLoop> {
        let (emit, events) = mpsc::channel();
        let (input_thread, input_stop) = threads::start_threads(emit.clone());

        Ok(EventLoop {
            emit: emit,
            events: events,
            terminal: trys!(Terminal::create(), "Failed to create terminal instance"),
            escape: trys!(Escape::create(), "Failed to create escape instance"),
            matches: Matches::from_iter(vec![]),
            selected: 0,
            query: Arc::new(format!("")),
            success: false,
            input_thread: Some(input_thread),
            input_stop: input_stop,
            search: None
        })
    }

    fn stop_threads(&mut self) -> StrResult<()> {
        // prompt the input thread to stop
        self.input_stop.store(true, Ordering::Relaxed);

        // insert a bogus byte to wake it up
        trys!(self.terminal.insert_input(" "), "Failed to insert bogus byte");

        // get our input thread handle
        let handle = match self.input_thread.take() {
            None => return Err(StrError::new("No input thread handle", None)),
            Some(handle) => handle
        };

        // join the thread
        match handle.join() {
            Ok(_) => Ok(()),
            Err(opaque) => Err({
                let error = opaque.downcast_ref::<StrError>().map(|error: &StrError| {
                    let dummy = StrError::new("dummy", None);
                    let value: Box<Error> = Box::new(dummy);
                    let raw_object: raw::TraitObject = unsafe {mem::transmute(value)};
                    let synthetic: Box<Error> = unsafe {mem::transmute(raw::TraitObject {
                        data: error as *const _ as *mut (),
                        vtable: raw_object.vtable
                    })};
                    synthetic
                });
                StrError::new("Input thread failed", error)
            })
        }
    }

    fn start_query(&self) {
        if !self.query.is_empty() {
            // only execute queries on non-empty queries
            match self.search {
                None => {}, // do nothing
                Some(ref base) => {
                    let emit = self.emit.clone();
                    let query = self.query.clone();
                    let base = base.clone();
                    thread::spawn(move || {
                        threads::start_query(emit, base, query);
                    });
                }
            }
        }
    }

    pub fn run(&mut self) -> StrResult<()> {
        // draw the prompt
        let size = self.terminal.rows() as usize;
        trys!(self.terminal.output_str(self.escape.render_prompt(size)),
              "Failed to render prompt");

        // flush the terminal
        trys!(self.terminal.flush(), "Failed to flush terminal");

        for event in self.events.iter() {
            match event {
                Event::SearchReady(base) => {
                    self.search = Some(Arc::new(base));
                    self.start_query();
                },
                Event::Input(chr) => {
                    Arc::make_mut(&mut self.query).push(chr);
                    trys!(self.terminal.output_str(self.escape.query_output(chr)),
                          "Failed to output character");
                    self.start_query();
                },
                Event::Match(matches, query) => {
                    debug!("Got match event: {:?}, {:?}", matches, query);
                    if query == self.query {
                        // only draw matches for the current query
                        self.selected = 0;
                        self.matches = Matches::from_iter(matches);
                        let size = self.terminal.cols() as usize;
                        trys!(self.terminal.output_str(
                            self.escape.matches_output(
                                &self.matches,
                                size,
                                self.selected)),
                              "Failed to output matches");
                    }
                },
                Event::Quit(success) => {
                    debug!("Got quit event: {:?}", success);
                    self.success = success;
                    break;
                },
                Event::KeyDown => {
                    self.selected = cmp::min(self.selected + 1, self.matches.len());
                },
                Event::KeyUp => {
                    self.selected = cmp::max(1, self.selected) - 1;
                },
                Event::Clear => {
                    if !self.query.is_empty() {
                        trys!(self.terminal.output_str(
                            self.escape.move_back(self.query.len())),
                              "Failed to output to terminal");
                        self.query = Arc::new(format!(""));
                    } else {
                        trys!(self.emit.send(Event::Bell),
                              "Failed to send bell event");
                    }
                },
                Event::Bell => {
                    trys!(self.terminal.output_str(self.escape.bell()),
                          "Failed to output bell");
                }
            }

            debug!("Flushing output");
            trys!(self.terminal.flush(), "Failed to flush terminal");
        }

        // stop the input thread
        trys!(self.stop_threads(), "Failed to stop threads");

        // draw the best match if it exists
        trys!(self.terminal.output_str(
            self.escape.best_match_output(&self.matches)),
              "Failed to draw best match");

        debug!("Flushing output");
        trys!(self.terminal.flush(), "Failed to flush terminal");

        // insert the successful match onto the terminal input buffer
        if self.success {
            match self.matches.best() {
                None => {debug!("No best match")},
                Some(m) => {
                    trys!(self.terminal.insert_input::<&String>(m.borrow()),
                          "Failed to insert input");
                }
            }
        }
        
        // success
        Ok(())
    }
}
