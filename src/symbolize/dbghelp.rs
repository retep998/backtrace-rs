// Copyright 2014-2015 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

#![allow(bad_style)]

use std::ffi::OsString;
use std::mem;
use std::os::windows::prelude::*;
use std::slice;
use kernel32;
use winapi::*;

use Symbol;

struct SymbolInfo<'a> {
    info: &'a SYMBOL_INFOW,
    data: Option<&'a [u8]>,
    line: Option<&'a IMAGEHLP_LINEW64>,
    line_data: Option<&'a [u8]>,
}

impl<'a> Symbol for SymbolInfo<'a> {
    fn name(&self) -> Option<&[u8]> { self.data }
    fn addr(&self) -> Option<*mut c_void> { Some(self.info.Address as *mut _) }
    fn filename(&self) -> Option<&[u8]> { self.line_data }
    fn lineno(&self) -> Option<u32> { self.line.map(|l| l.LineNumber as u32) }
}

pub fn resolve(addr: *mut c_void, cb: &mut FnMut(&Symbol)) {
    // According to windows documentation, all dbghelp functions are
    // single-threaded.
    let _g = ::lock::lock();

    unsafe {
        let size = 2 * MAX_SYM_NAME + mem::size_of::<SYMBOL_INFOW>();
        let mut data = vec![0u8; size];
        let info = &mut *(data.as_mut_ptr() as *mut SYMBOL_INFOW);
        info.MaxNameLen = MAX_SYM_NAME as ULONG;
        // the struct size in C.  the value is different to
        // `size_of::<SYMBOL_INFOW>() - MAX_SYM_NAME + 1` (== 81)
        // due to struct alignment.
        info.SizeOfStruct = 88;

        let _c = ::dbghelp_init();

        let mut displacement = 0u64;
        let ret = ::dbghelp::SymFromAddrW(kernel32::GetCurrentProcess(),
                                          addr as DWORD64,
                                          &mut displacement,
                                          info);
        if ret != TRUE {
            return
        }
        let name = slice::from_raw_parts(info.Name.as_ptr() as *const u16,
                                         info.NameLen as usize);
        let name = OsString::from_wide(name);
        let name = name.to_str().map(|s| s.as_bytes());

        let mut line = mem::zeroed::<IMAGEHLP_LINEW64>();
        line.SizeOfStruct = mem::size_of::<IMAGEHLP_LINEW64>() as DWORD;
        let mut displacement = 0;
        let ret = ::dbghelp::SymGetLineFromAddrW64(kernel32::GetCurrentProcess(),
                                                   addr as DWORD64,
                                                   &mut displacement,
                                                   &mut line);
        let line_data;
        let (line, line_data) = if ret == TRUE {
            let base = line.FileName;
            let mut len = 0;
            while *base.offset(len) != 0 {
                len += 1;
            }
            let name = slice::from_raw_parts(base, len as usize);
            line_data = OsString::from_wide(name);
            (Some(&line), line_data.to_str().map(|s| s.as_bytes()))
        } else {
            (None, None)
        };

        cb(&SymbolInfo {
            info: info,
            data: name,
            line: line,
            line_data: line_data,
        })
    }
}
