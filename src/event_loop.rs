
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
            emit: emit.clone(),
            events: events,
            terminal: trys!(Terminal::create(), "Failed to create terminal instance"),
            ui: trys!(UI::create(emit), "Failed to create UI instance")
        })
    }

    fn start_threads(&self) -> (JoinHandle<()>, Arc<AtomicBool>) {
        // start the sigint thread
        let emit = self.emit.clone();
        thread::spawn(move || {
            threads::read_signals(emit);
        });

        // start reading history
        let emit = self.emit.clone();
        thread::spawn(move || {
            threads::read_history(emit);
        });

        // start the input thread
        let input_stop = Arc::new(AtomicBool::new(false));
        let emit = self.emit.clone();
        let stop = input_stop.clone();
        let input_thread = thread::spawn(|| {
            threads::read_input(emit, stop)
        });

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
        if !query.is_empty() {
            // only execute queries on non-empty queries
            search.map(|base| {
                let emit = self.emit.clone();
                thread::spawn(move || {
                    threads::start_query(emit, base, query);
                });
            });
        }
    }

    pub fn run(&mut self) -> StrResult<()> {
        // start the threads
        let (input_thread, input_stop) = self.start_threads();

        // draw the prompt
        trys!(self.terminal.output_str(self.ui.render_prompt()), "Failed to render prompt");

        // flush the terminal
        trys!(self.terminal.flush(), "Failed to flush terminal");

        let mut query = Arc::new(format!(""));
        let mut escape = None;
        let mut matches = None;
        let mut search = None;
        let mut success = false;
        let mut match_number = 0;

        for event in self.events.iter() {
            match event {
                Event::SearchReady(base) => {
                    search = Some(Arc::new(base));
                    self.start_query(search.clone(), query.clone());
                },
                Event::Query(q) => {
                    query = Arc::new(q);
                    escape = None;
                    matches = None;
                    self.start_query(search.clone(), query.clone());
                }
                Event::Input(chr) => {
                    debug!("Got input event: {:?}", chr);
                    let (output, new_escape) = trys!(self.ui.input_chr::<&String, &String>(query.borrow(), escape.as_ref(), chr),
                                                     "Failed to input character");
                    trys!(self.terminal.output_str(output),
                          "Failed to write output");

                    escape = new_escape;
                },
                Event::Match(m, q) => {
                    debug!("Got match event: {:?}, {:?}", m, q);
                    if q == query {
                        // only draw matches for the current query
                        match_number = 0;
                        trys!(self.terminal.output_str(self.ui.render_matches(Some(m.as_ref()), match_number)),
                              "Failed to draw matches");
                        matches = Some(m);
                    }
                },
                Event::Quit(s) => {
                    debug!("Got quit event: {:?}", s);
                    success = s;
                    break;
                },
                Event::MatchDown => {
                    trys!(self.terminal.output_str(
                        trys!(self.ui.match_down(match_number, matches.as_ref().map(|m| m.len())),
                              "Failed to match down")),
                          "Failed to output to terminal");
                },
                Event::MatchUp => {
                    trys!(self.terminal.output_str(
                        trys!(self.ui.match_up(match_number),
                              "Failed to match down")),
                          "Failed to output to terminal");
                },
                Event::Select(number) => {
                    trys!(self.terminal.output_str(self.ui.clear_screen()), "Failed to clear screen");
                    trys!(self.terminal.output_str(self.ui.render_matches(matches.as_ref().map(|m| m.as_ref()), number)),
                          "Failed to draw matches");
                    match_number = number;
                }
            }

            debug!("Flushing output");
            trys!(self.terminal.flush(), "Failed to flush terminal");
        }

        // stop the input thread
        trys!(self.stop_threads(input_thread, input_stop), "Failed to stop threads");

        // draw the best match if it exists
        match matches {
            Some(ref m) => {
                trys!(self.terminal.output_str(self.ui.render_best_match::<&String>(m[match_number].borrow())),
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
            try!(matches.map_or(Ok(()), |m| {
                self.terminal.insert_input::<&String>(m[match_number].borrow()).or_else(|e| {
                    errs!(e, "Failed to insert input")
                })
            }));
        }

        // success
        Ok(())
    }
}
