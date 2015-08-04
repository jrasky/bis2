use std::borrow::Cow;

use search::SearchBase;

#[derive(Debug)]
pub enum Event {
    SearchReady(SearchBase),
    Input(char),
    Match(Vec<Cow<'static, str>>, Cow<'static, str>),
}
