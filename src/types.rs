use std::sync::Arc;

use search::SearchBase;

#[derive(Debug)]
pub enum Event {
    SearchReady(SearchBase),
    Input(char),
    Match(Vec<Arc<String>>, String),
    Quit(bool)
}
