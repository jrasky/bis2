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
extern crate unicode_width;
extern crate term;
extern crate libc;
#[macro_use]
extern crate log;
extern crate env_logger;
extern crate threadpool;
extern crate flx;
extern crate serde_json;
#[macro_use]
extern crate serde_derive;
extern crate dirs;

use std::mem;

use event_loop::EventLoop;

mod constants;
mod types;
mod bis_c;
mod terminal;
mod ui;
mod threads;
mod event_loop;

fn main() {
    // init logging
    env_logger::init();

    // create the event loop
    let mut ev_loop = EventLoop::create();

    // run the event loop
    ev_loop.run();

    // destroy the event loop
    mem::drop(ev_loop);

    // done
}
