use std::io::prelude::*;

use std::io::BufReader;
use std::sync::Arc;
use std::sync::mpsc::Sender;
use std::sync::atomic::{AtomicBool, Ordering};
use std::fs::File;
use std::iter::FromIterator;
use std::borrow::Borrow;

use std::env;
use std::io;

use search::{SearchBase, LineInfo};
use bis_c;

use types::*;

pub fn read_history(emit: Sender<Event>) {
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
}

pub fn read_input(emit: Sender<Event>, stop: Arc<AtomicBool>) {
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
}

pub fn read_signals(emit: Sender<Event>) {
    // wait for a sigint
    bis_c::wait_sigint().expect("Failed to wait for sigint");

    debug!("Caught sigint");

    // send a quit signal
    emit.send(Event::Quit(false)).is_ok();
    // might happen after events is closed, so don't fail
}

pub fn start_query(emit: Sender<Event>, base: Arc<SearchBase>, query: Arc<String>) {
    let result = base.query::<&String>(query.borrow());
    emit.send(Event::Match(result, query)).and_then(|_| {
        trace!("Finished query");
        Ok(())
    }).is_ok();
    // don't panic on fail send, events might be already closed
}
