use std::sync::mpsc::{Sender, Receiver, channel};

#[derive(Debug, Clone)]
struct LatchHandle {
    emit: Sender<()>
}

#[derive(Debug)]
struct Latch {
    count: usize,
    handle: LatchHandle,
    events: Receiver<()>
}

unsafe impl Send for LatchHandle {}

impl LatchHandle {
    fn count_down(&self) -> bool {
        self.emit.send(()).is_ok()
    }
}

impl Latch {
    fn new(count: usize) -> Latch {
        let (emit, events) = channel();

        Latch {
            count: count,
            handle: LatchHandle {
                emit: emit
            },
            events: events
        }
    }

    fn new_handle(&self) -> LatchHandle {
        self.handle.clone()
    }

    fn process_queue(&mut self) {
        loop {
            match self.events.try_recv() {
                Ok(()) => {
                    if self.count > 0 {
                        self.count -= 1;
                    }
                },
                Err(_) => break
            }
        }
    }

    fn count_down(&mut self) {
        self.process_queue();

        if self.count > 0 {
            self.count -= 1;
        }
    }

    fn poll(&mut self) -> usize {
        self.process_queue();
        self.count
    }

    fn wait(&mut self) {
        loop {
            match self.events.recv() {
                Ok(()) => {
                    self.count -= 1;
                    if self.count == 0 {
                        break;
                    }
                },
                Err(_) => break
            }
        }
    }
}
