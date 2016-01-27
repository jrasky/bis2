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
use std::sync::Arc;

use flx::SearchBase;

#[derive(Debug)]
pub enum Event {
    SearchReady(SearchBase),
    Input(char),
    Match(Vec<Arc<String>>, Arc<String>),
    Quit(bool),
    KeyUp,
    KeyDown,
    Clear,
    Backspace,
    Bell,
}

impl Event {
    pub fn maybe_clone(&self) -> Option<Event> {
        use ::types::Event::*;
        match self {
            &SearchReady(_) => None,
            &Input(ref chr) => Some(Input(*chr)),
            &Match(ref matches, ref query) => Some(Match(matches.clone(), query.clone())),
            &Quit(ref success) => Some(Quit(*success)),
            &KeyUp => Some(KeyUp),
            &KeyDown => Some(KeyDown),
            &Clear => Some(Clear),
            &Backspace => Some(Backspace),
            &Bell => Some(Bell),
        }
    }
}
