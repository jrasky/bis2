use types::*;

struct EventLoop {
    emit: Emitter,
    mutex: Arc<Mutex<()>>
}

impl EventLoop {
    fn new() -> EventLoop {
        let mutex = Arc::new(Mutex::new(()));
        let mut latch = Latch::new(1);
        let (emit, events) = channel();

        let handle = latch.new_handle();
        let inner_mutex = mutex.clone();
        let inner_emit = emit.clone();

        thread::spawn(move || {
            // lock the mutex
            let lock = match inner_mutex.lock() {
                Ok(lock) => lock,
                Err(e) => {
                    panic!("Failed to lock mutex: {}", e);
                }
            };

            // count down the latch
            handle.count_down();

            // thread state
            let mut query = String::default();

            loop {
                match events.recv() {
                    Err(_) => break,
                    Ok(event) => {
                        debug!("Got event: {:?}", event);
                        match event {
                            Event::Input(chr) => {
                                // UI::input_char
                                // SearchBase::query
                            },
                            Event::Match(matches, query) => {
                                // UI::render_matches
                                // Terminal::draw_matches
                            },
                            Event::Quit(success) => {
                                // quit the event loop
                                break
                            }
                        }
                    }
                }
            }

            // release the lock
            mem::drop(lock);
        });

        // wait for the loop to start
        latch.wait();

        EventLoop {
            emit: emit,
            mutex: mutex
        }
    }
}
