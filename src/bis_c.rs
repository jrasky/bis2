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

use std::ffi::CString;

use error::StrResult;

mod c {
    use libc::*;

    use std::ffi;
    use std::io;

    use error::StrError;

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
        pub fn bis_mask_sigint() -> c_int;
        pub fn bis_wait_sigint() -> c_int;
        pub fn bis_insert_input(input: *const c_char) -> c_int;
    }

    pub unsafe fn get_bis_error() -> StrError {
        let error_cstr = ffi::CStr::from_ptr(bis_error_info.error_str);
        StrError::new(error_cstr.to_string_lossy().into_owned(),
                      match bis_error_info.is_errno {
                          1 => Some(Box::new(io::Error::last_os_error())),
                          _ => None,
                      })
    }
}

pub fn prepare_terminal() -> StrResult<()> {
    debug!("Preparing terminal");
    match unsafe { c::bis_prepare_terminal() } {
        0 => Ok(()),
        _ => Err(unsafe { c::get_bis_error() }),
    }
}

pub fn restore_terminal() -> StrResult<()> {
    debug!("Restoring terminal");
    match unsafe { c::bis_restore_terminal() } {
        0 => Ok(()),
        _ => Err(unsafe { c::get_bis_error() }),
    }
}

pub fn mask_sigint() -> StrResult<()> {
    debug!("Masking sigint");
    match unsafe { c::bis_mask_sigint() } {
        0 => Ok(()),
        _ => Err(unsafe { c::get_bis_error() }),
    }
}

pub fn wait_sigint() -> StrResult<()> {
    debug!("Waiting for sigint");
    match unsafe { c::bis_wait_sigint() } {
        0 => Ok(()),
        _ => Err(unsafe { c::get_bis_error() }),
    }
}

pub fn insert_input<T: Into<Vec<u8>>>(input: T) -> StrResult<()> {
    let cstr = match CString::new(input) {
        Ok(s) => s,
        Err(e) => return errs!(e, "Failed to create CString"),
    };

    match unsafe { c::bis_insert_input(cstr.as_ptr()) } {
        0 => {}
        _ => return Err(unsafe { c::get_bis_error() }),
    }

    // return success
    Ok(())
}

pub fn get_terminal_size() -> StrResult<(u16, u16)> {
    debug!("Getting terminal size");
    let mut term_size = c::bis_term_size_t { rows: 0, cols: 0 };

    match unsafe { c::bis_get_terminal_size(&mut term_size) } {
        0 => Ok((term_size.rows, term_size.cols)),
        _ => Err(unsafe { c::get_bis_error() }),
    }
}
