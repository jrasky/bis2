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
use threadpool::ThreadPool;

use std::sync::{Arc, Mutex};
use std::sync::mpsc::{Sender, Receiver};
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::JoinHandle;
use std::iter::FromIterator;
use std::fs::File;

use std::sync::mpsc;
use std::env;
use std::thread;

use terminal::Terminal;
use flx::SearchBase;
use dirs;
use threads;
use serde_json;

use ui::*;
use types::*;
use constants::*;

pub struct EventLoop {
    emit: Sender<Event>,
    events: Receiver<Event>,
    terminal: Terminal,
    escape: Escape,
    matches: Matches,
    selected: usize,
    query: String,
    success: bool,
    input_thread: Option<JoinHandle<()>>,
    input_stop: Arc<AtomicBool>,
    search: Option<Arc<SearchBase>>,
    pool: ThreadPool,
    recent: Vec<String>,
    completions: Option<Arc<Mutex<Completions>>>,
}

impl EventLoop {
    pub fn create() -> EventLoop {
        let (emit, events) = mpsc::channel();
        let (input_thread, input_stop) = threads::start_threads(emit.clone());

        EventLoop {
            emit: emit,
            events: events,
            terminal: Terminal::create(),
            escape: Escape::create(),
            matches: Matches::from_iter(vec![]),
            selected: 0,
            query: "".into(),
            success: false,
            input_thread: Some(input_thread),
            input_stop: input_stop,
            search: None,
            pool: ThreadPool::new(NUM_THREADS),
            recent: vec![],
            completions: None,
        }
    }

    fn stop_threads(&mut self) {
        // prompt the input thread to stop
        self.input_stop.store(true, Ordering::Relaxed);

        // get our input thread handle
        let handle = self.input_thread.take().expect("No input thread handle");

        // join the thread
        handle.join().expect("Input thread failed");
    }

    fn start_query(&self) {
        if !self.query.is_empty() {
            // only execute queries on non-empty queries
            match self.search {
                None => {} // do nothing
                Some(ref base) => {
                    let emit = self.emit.clone();
                    let query = self.query.clone();
                    let base = base.clone();
                    self.pool.execute(move || {
                        threads::start_query(emit, base, query);
                    });
                }
            }
        }
    }

    pub fn run(&mut self) {
        // draw the prompt
        let size = self.terminal.rows() as usize;
        self.terminal.output_str(self.escape.render_prompt(size));

        self.terminal.flush();

        for event in self.events.iter() {
            match event {
                Event::CompletionsReady(completions) => {
                    // put the completions in a refcell
                    let guard = Arc::new(Mutex::new(completions));

                    // use a lifetime boundary as to clarify the situation to rustc
                    let history_emit = self.emit.clone();
                    let history_guard = guard.clone();

                    thread::spawn(move || {
                        let history_completions = if let Ok(completions) = history_guard.try_lock() {
                            completions
                        } else {
                            // the main thread grabbed the lock first, and is probably exiting
                            // do so ourself
                            debug!("Completions thread failed to grab completions lock");
                            return
                        };

                        threads::read_history(history_completions, history_emit);
                    });

                    // save the completions so we can use them later
                    self.completions = Some(guard);
                }
                Event::HistoryReady(recent) => {
                    self.recent = recent;
                    if self.query.is_empty() {
                        self.matches = Matches::from_iter(self.recent.iter().cloned());
                        self.selected = 0;
                        let size = self.terminal.cols() as usize;
                        self.terminal.output_str(self.escape.matches_output(&self.matches, size, self.selected));
                    }
                }
                Event::SearchReady(base) => {
                    self.search = Some(Arc::new(base));
                    self.start_query();
                }
                Event::Input(chr) => {
                    self.query.push(chr);
                    self.terminal.output_str(self.escape.query_output(chr));
                    self.start_query();
                }
                Event::Match(matches, query) => {
                    debug!("Got match event: {:?}, {:?}", matches, query);
                    if query == self.query {
                        // only draw matches for the current query
                        self.selected = 0;
                        self.matches = Matches::from_iter(matches);
                        let size = self.terminal.cols() as usize;
                        self.terminal.output_str(self.escape.matches_output(&self.matches, size, self.selected));
                    }
                }
                Event::Quit(success) => {
                    debug!("Got quit event: {:?}", success);
                    self.success = success;
                    break;
                }
                Event::KeyDown => {
                    if self.selected + 1 < self.matches.len() {
                        self.selected += 1;
                        let size = self.terminal.cols() as usize;
                        self.terminal.output_str(self.escape.match_down(&self.matches, size, self.selected));
                    } else {
                        self.emit.send(Event::Bell).unwrap();
                    }
                }
                Event::KeyUp => {
                    if self.selected > 0 {
                        self.selected -= 1;
                        let size = self.terminal.cols() as usize;
                        self.terminal.output_str(self.escape.match_up(&self.matches, size, self.selected));
                    } else {
                        self.emit.send(Event::Bell).unwrap();
                    }
                }
                Event::Clear => {
                    if !self.query.is_empty() {
                        self.terminal.output_str(self.escape.move_back(self.query.len()));
                        self.query = "".into();
                        self.matches = Matches::from_iter(self.recent.iter().cloned());
                        self.selected = 0;
                        let size = self.terminal.cols() as usize;
                        self.terminal.output_str(self.escape.matches_output(&self.matches, size, self.selected));
                    } else {
                        self.emit.send(Event::Bell).unwrap();
                    }
                }
                Event::Backspace => {
                    if !self.query.is_empty() {
                        self.query.pop();
                        self.terminal.output_str(self.escape.move_back(1));
                        if !self.query.is_empty() {
                            self.start_query();
                        } else {
                            self.matches = Matches::from_iter(self.recent.iter().cloned());
                            let size = self.terminal.cols() as usize;
                            self.terminal.output_str(self.escape.matches_output(&self.matches, size, self.selected));
                        }
                    } else {
                        self.emit.send(Event::Bell).unwrap();
                    }
                }
                Event::Bell => {
                    self.terminal.output_str(self.escape.bell());
                }
            }

            debug!("Flushing output");
            self.terminal.flush();
        }

        // stop the input thread
        self.stop_threads();

        // draw the best match if it exists
        self.terminal.output_str(self.escape.best_match_output(&self.matches, self.selected, self.query.is_empty()));

        debug!("Flushing output");
        self.terminal.flush();

        // insert the successful match onto the terminal input buffer
        if self.success {
            match self.matches.get(self.selected) {
                None => debug!("No best match"),
                Some(m) => {
                    if let Some(ref completions) = self.completions {
                        if let Ok(path) = env::current_dir() {
                            if let Ok(mut completions) = completions.try_lock() {
                                completions.add_completion(m.get().clone(), path);

                                // try to save them
                                let mut completions_path = dirs::home_dir().unwrap_or("".into());
                                completions_path.push(".bis2_completions");

                                trace!("Completions path: {:?}", completions_path);

                                match File::create(completions_path) {
                                    Ok(mut file) => {
                                        if let Err(error) = serde_json::to_writer(&mut file, &*completions) {
                                            warn!("Failed to save completions: {}", error);
                                        }
                                    }
                                    Err(error) => {
                                        warn!("Failed to open completions file: {}", error);
                                    }
                                }
                            } else {
                                // the other thread hasn't released completions yet
                                debug!("Failed to lock completions object");
                            }
                        }
                    }

                    self.terminal.insert_input(m.get());
                }
            }
        }
    }
}
