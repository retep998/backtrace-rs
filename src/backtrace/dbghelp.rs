// Copyright 2014 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

#![allow(bad_style)]

use std::mem;
use winapi::*;
use kernel32;

use Frame;

impl Frame for STACKFRAME64 {
    fn ip(&self) -> *mut c_void { self.AddrPC.Offset as *mut _ }
    fn symbol_address(&self) -> *mut c_void { self.ip() }
}

#[inline(always)]
pub fn trace(cb: &mut FnMut(&Frame) -> bool) {
    // According to windows documentation, all dbghelp functions are
    // single-threaded.
    let _g = ::lock::lock();

    unsafe {
        // Allocate necessary structures for doing the stack walk
        let process = kernel32::GetCurrentProcess();
        let thread = kernel32::GetCurrentThread();

        // The CONTEXT structure needs to be aligned on a 16-byte boundary for
        // 64-bit Windows, but currently we don't have a way to express that in
        // Rust. Allocations are generally aligned to 16-bytes, though, so we
        // box this up.
        let mut context = Box::new(mem::zeroed::<CONTEXT>());
        kernel32::RtlCaptureContext(&mut *context);
        let mut frame: STACKFRAME64 = mem::zeroed();
        let image = init_frame(&mut frame, &context);

        // Initialize this process's symbols
        let _c = ::dbghelp_init();

        // And now that we're done with all the setup, do the stack walking!
        while ::dbghelp::StackWalk64(image as DWORD, process, thread, &mut frame,
                                     &mut *context as *mut _ as *mut _,
                                     None,
                                     Some(::dbghelp::SymFunctionTableAccess64),
                                     Some(::dbghelp::SymGetModuleBase64),
                                     None) == TRUE {
            if frame.AddrPC.Offset == frame.AddrReturn.Offset ||
               frame.AddrPC.Offset == 0 ||
               frame.AddrReturn.Offset == 0 {
                break
            }

            if !cb(&frame) {
                break
            }
        }
    }
}

#[cfg(target_arch = "x86_64")]
fn init_frame(frame: &mut STACKFRAME64, ctx: &CONTEXT) -> WORD {
    frame.AddrPC.Offset = ctx.Rip as u64;
    frame.AddrPC.Mode = ADDRESS_MODE::AddrModeFlat;
    frame.AddrStack.Offset = ctx.Rsp as u64;
    frame.AddrStack.Mode = ADDRESS_MODE::AddrModeFlat;
    frame.AddrFrame.Offset = ctx.Rbp as u64;
    frame.AddrFrame.Mode = ADDRESS_MODE::AddrModeFlat;
    IMAGE_FILE_MACHINE_AMD64
}

#[cfg(target_arch = "x86")]
fn init_frame(frame: &mut STACKFRAME64, ctx: &CONTEXT) -> WORD {
    frame.AddrPC.Offset = ctx.Eip as u64;
    frame.AddrPC.Mode = ADDRESS_MODE::AddrModeFlat;
    frame.AddrStack.Offset = ctx.Esp as u64;
    frame.AddrStack.Mode = ADDRESS_MODE::AddrModeFlat;
    frame.AddrFrame.Offset = ctx.Ebp as u64;
    frame.AddrFrame.Mode = ADDRESS_MODE::AddrModeFlat;
    IMAGE_FILE_MACHINE_I386
}
