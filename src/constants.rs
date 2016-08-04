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
pub const MATCH_NUMBER: usize = 10;
pub const NUM_THREADS: usize = 4;
pub const COMPLETION_SCORE_FACTOR: f32 = 10.0;

pub const EOT: char = '\u{4}';
pub const CTRL_U: char = '\u{15}';
pub const CTRL_R: char = '\u{12}';
pub const CTRL_S: char = '\u{13}';
pub const ESC: char = '\u{1b}';
pub const BEL: char = '\u{7}';
pub const DEL: char = '\u{7f}';
pub const BSPC: char = '\u{8}';

pub const PROMPT: &'static str = "Match: ";
pub const FINISH: &'static str = " -> ";
pub const MATCH_SELECT: &'static str = "-> ";
pub const MATCH_PRE: &'static str = "\n";
