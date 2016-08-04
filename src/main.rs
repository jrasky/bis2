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
#![feature(io)]
#![feature(raw)]
#![feature(alloc)]
#![feature(custom_derive, plugin)]
#![plugin(serde_macros)]
extern crate unicode_width;
extern crate term;
extern crate libc;
#[macro_use]
extern crate log;
extern crate env_logger;
extern crate threadpool;
extern crate flx;
extern crate alloc;
extern crate serde_json;

use std::mem;

use event_loop::EventLoop;

mod constants;
#[macro_use]
mod error;
mod types;
mod bis_c;
mod terminal;
mod ui;
mod threads;
mod event_loop;

fn main() {
    // init logging
    match env_logger::init() {
        Ok(_) => {
            trace!("Successfully initialized logging");
        }
        Err(e) => {
            panic!("Failed to initialize logging: {}", e);
        }
    }

    // create the event loop
    let mut ev_loop = match EventLoop::create() {
        Ok(ev) => ev,
        Err(e) => panic!("Failed to create event loop: {}", e),
    };

    // run the event loop
    match ev_loop.run() {
        Ok(_) => {
            debug!("Event loop exited successfully");
        }
        Err(e) => {
            panic!("Event loop failed: {}", e);
        }
    }

    // destroy the event loop
    mem::drop(ev_loop);

    // done
}
