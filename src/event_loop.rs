
use std::io::prelude::*;

use std::sync::Arc;
use std::sync::mpsc::{Sender, Receiver};
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::JoinHandle;
use std::borrow::Borrow;

use std::sync::mpsc;
use std::thread;

use ui::UI;
use terminal::Terminal;
use search::SearchBase;
use threads;

use error::*;
use types::*;

pub struct EventLoop {
    emit: Sender<Event>,
    events: Receiver<Event>,
    terminal: Terminal,
    ui: UI
}

impl EventLoop {
    pub fn create() -> StrResult<EventLoop> {
        let (emit, events) = mpsc::channel();

        Ok(EventLoop {
            emit: emit,
            events: events,
            terminal: trys!(Terminal::create(), "Failed to create terminal instance"),
            ui: trys!(UI::create(), "Failed to create UI instance")
        })
    }

    fn start_threads(&self) -> (JoinHandle<()>, Arc<AtomicBool>) {
        // start the sigint thread
        {
            let emit = self.emit.clone();
            thread::spawn(move || {
                threads::read_signals(emit);
            });
        }

        // start reading history
        {
            let emit = self.emit.clone();
            thread::spawn(move || {
                threads::read_history(emit);
            });
        }

        // start the input thread
        let input_thread;
        let input_stop = Arc::new(AtomicBool::new(false));
        {
            let emit = self.emit.clone();
            let stop = input_stop.clone();
            input_thread = thread::spawn(move || {
               threads::read_input(emit, stop)
            });
        }

        (input_thread, input_stop)
    }

    fn stop_threads(&mut self, input_thread: JoinHandle<()>, input_stop: Arc<AtomicBool>) -> StrResult<()> {
        // prompt the input thread to stop
        input_stop.store(true, Ordering::Relaxed);

        // insert a bogus byte to wake it up
        trys!(self.terminal.insert_input(" "), "Failed to insert bogus byte");

        // join the thread
        input_thread.join().or_else(|e| {errs!(StrError::from_any(e), "Failed to wait for input thread")})
    }

    fn start_query(&self, search: Option<Arc<SearchBase>>, query: Arc<String>) {
        search.map(|base| {
            let emit = self.emit.clone();
            thread::spawn(move || {
                threads::start_query(emit, base, query);
            });
        });
    }

    pub fn run(&mut self) -> StrResult<()> {
        // start the threads
        let (input_thread, input_stop) = self.start_threads();

        // draw the prompt
        trys!(self.terminal.output_str(self.ui.render_prompt()), "Failed to render prompt");

        // flush the terminal
        trys!(self.terminal.flush(), "Failed to flush terminal");

        let mut query = Arc::new(format!(""));
        let mut best_match = None;
        let mut search = None;
        let mut success = false;
        let match_number = 0;

        loop {
            match self.events.recv() {
                Err(_) => break,
                Ok(event) => {
                    match event {
                        Event::SearchReady(base) => {
                            search = Some(Arc::new(base));
                            // execute a query if it isn't empty
                            if !query.is_empty() {
                                self.start_query(search.clone(), query.clone());
                            }
                        },
                        Event::Query(q) => {
                            query = Arc::new(q);
                            if !query.is_empty() {
                                self.start_query(search.clone(), query.clone());
                            } else {
                                best_match = None;
                            }
                        }
                        Event::Input(chr) => {
                            debug!("Got input event: {:?}", chr);
                            trys!(self.terminal.output_str(self.ui.input_char::<&String>(
                                self.emit.clone(), query.borrow(), chr)),
                                  "Failed to write output");
                        },
                        Event::Match(matches, q) => {
                            debug!("Got match event: {:?}, {:?}", matches, q);
                            if q == query {
                                best_match = matches.first().cloned();
                                // only draw matches for the current query
                                trys!(self.terminal.output_str(self.ui.render_matches(matches, match_number)),
                                      "Failed to draw matches");
                            }
                        },
                        Event::Quit(s) => {
                            debug!("Got quit event: {:?}", s);
                            success = s;
                            break;
                        }
                    }

                    debug!("Flushing output");
                    trys!(self.terminal.flush(), "Failed to flush terminal");
                }
            }
        }

        // stop the input thread
        trys!(self.stop_threads(input_thread, input_stop), "Failed to stop threads");

        // draw the best match if it exists
        match best_match {
            Some(ref m) => {
                trys!(self.terminal.output_str(self.ui.render_best_match::<&String>(m.borrow())),
                      "Failed to draw best match");
            },
            None => {
                trys!(self.terminal.output_str("\n"), "Failed to draw newline");
            }
        }

        debug!("Flushing output");
        trys!(self.terminal.flush(), "Failed to flush terminal");

        // insert the successful match onto the terminal input buffer
        if success {
            try!(best_match.map_or(Ok(()), |m| {
                self.terminal.insert_input::<&String>(m.borrow()).or_else(|e| {
                    errs!(e, "Failed to insert input")
                })
            }));
        }

        // success
        Ok(())
    }
}
