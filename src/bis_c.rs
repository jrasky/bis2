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

// bindings into bis_c.c

mod c {
    use libc::*;

    use std::ffi;
    use std::io;

    #[repr(C)]
    struct bis_error_info_t {
        error_str: *const c_char,
        is_errno: c_char,
    }

    #[repr(C)]
    pub struct bis_term_size_t {
        pub rows: c_ushort,
        pub cols: c_ushort,
    }

    extern "C" {
        static mut bis_error_info: bis_error_info_t;

        pub fn bis_prepare_terminal() -> c_int;
        pub fn bis_restore_terminal() -> c_int;
        pub fn bis_get_terminal_size(size: *mut bis_term_size_t) -> c_int;
    }

    pub fn c_panic() -> ! {
        unsafe {
            let error = ffi::CStr::from_ptr(bis_error_info.error_str).to_string_lossy();

            if bis_error_info.is_errno == 1 {
                panic!("{}: {}", error, io::Error::last_os_error());
            } else {
                panic!("{}", error);
            }
        }
    }
}

pub fn prepare_terminal() {
    unsafe {
        debug!("Preparing terminal");

        if c::bis_prepare_terminal() != 0 {
            c::c_panic();
        }
    }
}

pub fn restore_terminal() {
    unsafe {
        debug!("Restoring terminal");

        if c::bis_restore_terminal() != 0 {
            c::c_panic();
        }
    }
}

pub fn get_terminal_size() -> (u16, u16) {
    unsafe {
        debug!("Getting terminal size");
        let mut term_size = c::bis_term_size_t { rows: 0, cols: 0 };

        if c::bis_get_terminal_size(&mut term_size) != 0 {
            c::c_panic();
        }

        (term_size.rows, term_size.cols)
    }
}
