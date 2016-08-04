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
use std::collections::HashMap;
use std::path::PathBuf;

use flx::SearchBase;

use constants::*;

#[derive(Debug)]
pub enum Event {
    CompletionsReady(Completions),
    HistoryReady(Vec<Arc<String>>),
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

// I'm pretty sure HashMap is Send, as is String
#[derive(Debug, Serialize, Deserialize)]
pub struct Completions {
    // Map<Line, Vec<Path>>
    info: HashMap<String, Vec<(PathBuf, f32)>>
}

impl Completions {
    pub fn new() -> Completions {
        Completions {
            info: HashMap::new()
        }
    }

    pub fn get_score(&self, line: &String, path: &PathBuf) -> f32 {
        let path_count = path.components().count() as f32;

        if let Some(paths) = self.info.get(line) {
            paths.iter().map(|&(ref score_path, count)| {
                let base_count = path.components()
                    .zip(score_path.components())
                    .take_while(|&(path_component, score_component)| path_component == score_component)
                    .count() as f32;

                let total_count = (2.0 * base_count) - path_count;

                if total_count > 0.0 {
                    total_count * count
                } else {
                    0.0
                }
            }).sum::<f32>() * COMPLETION_SCORE_FACTOR
        } else {
            0.0
        }
    }

    pub fn add_completion(&mut self, line: String, path: PathBuf) {
        let entry = self.info.entry(line).or_insert(vec![]);
        let mut count = 1.0;
        let mut place = None;

        for (idx, &(ref entry_path, entry_count)) in entry.iter().enumerate() {
            if &path == entry_path {
                count = entry_count + 1.0;
                place = Some(idx);
                break;
            }
        }

        entry.push((path, count));

        if let Some(idx) = place {
            // swap out with the old count
            entry.swap_remove(idx);
        }
    }
}

impl Event {
    pub fn maybe_clone(&self) -> Option<Event> {
        use ::types::Event::*;
        match self {
            &CompletionsReady(_) => None,
            &HistoryReady(_) => None,
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
