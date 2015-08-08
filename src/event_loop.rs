struct EventLoop {
    emit: Sender<Event>,
    events: Receiver<Event>,
    terminal: Terminal,
    ui: UI
}

impl EventLoop {
    fn create() -> StrResult<EventLoop> {
        let (emit, events) = mpsc::channel();

        Ok(EventLoop {
            emit: emit,
            events: events,
            terminal: trys!(Terminal::create(), "Failed to create terminal instance"),
            ui: trys!(UI::create(), "Failed to create UI instance");
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

    fn stop_threads(&mut self, input_thread: JoinHandle<()>, input_stop: Arc<AtomicBool>) {
        // prompt the input thread to stop
        input_stop.store(true, Ordering::Relaxed);

        // insert a bogus byte to wake it up
        trys!(self.terminal.insert_input(" "), "Failed to insert bogus byte");

        // join the thread
        trys!(input_thread.join(), "Failed to wait for input thread");
    }

    fn run(&mut self) -> StrResult<()> {
        // start the threads
        let (input_thread, input_stop) = self.start_threads();

        // draw the prompt
        trys!(terminal.output_str(ui.render_prompt(), "Failed to render prompt"));

        // flush the terminal
        trys!(terminal.flush(), "Failed to flush terminal");

        let mut query = Arc::new(format!(""));
        let best_match = None;
        let search = None;
        let success = false;
        let match_number = 0;

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
                            trys!(terminal.output_str(ui.input_char::<&String>(emit.clone(), query.borrow(), chr)),
                                  "Failed to write output");
                        },
                        Event::Match(matches, q) => {
                            debug!("Got match event: {:?}, {:?}", matches, q);
                            if q == query {
                                best_match = matches.first().cloned();
                                // only draw matches for the current query
                                trys!(terminal.output_str(ui.render_matches(matches, match_number)),
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
                    trys!(terminal.flush(), "Failed to flush terminal");
                }
            }
        }

        // stop the input thread
        trys!(self.stop_threads(), "Failed to stop threads");

        // draw the best match if it exists
        match best_match {
            Some(ref m) => {
                trys!(self.terminal.output_str(ui.render_best_match::<&String>(m.borrow())),
                      "Failed to draw best match");
            },
            None => {
                trys!(terminal.output_str("\n"), "Failed to draw newline");
            }
        }

        debug!("Flushing output");
        trys!(terminal.flush(), "Failed to flush terminal");

        if success {
            best_match.map(|m| {
                trys!(terminal.insert_input::<&String>(m.borrow()),
                      "Failed to insert input");
            });
        }
    }
}
