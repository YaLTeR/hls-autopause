#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]
#![allow(unused_assignments)]
#![feature(core_intrinsics)]
#![feature(drop_types_in_const)]
#![feature(plugin)]
#![plugin(interpolate_idents)]

extern crate kernel32;
#[macro_use]
extern crate lazy_static;
extern crate libc;
#[macro_use]
extern crate log;
extern crate psapi;
extern crate user32;
extern crate widestring;
extern crate winapi;

use std::thread;
use winapi::*;

#[macro_use]
mod macros;

mod features;
mod hookable;
mod hooks {
    pub mod engine;
    pub mod kernel32;
    pub mod server;
}
mod logger;
mod minhook;
mod moduleinfo;
use moduleinfo::ModuleInfo;
mod pattern;
mod utils;

const DLL_PROCESS_ATTACH: DWORD = 1;
const DLL_PROCESS_DETACH: DWORD = 0;

#[no_mangle]
pub extern "stdcall" fn DllMain(instance: HINSTANCE, reason: DWORD, _reserved: LPVOID) -> BOOL {
    match reason {
        DLL_PROCESS_ATTACH => {
            unsafe {
                kernel32::DisableThreadLibraryCalls(instance);
            }

            thread::spawn(main_thread);
        }
        DLL_PROCESS_DETACH => {
            minhook::uninitialize();
        }
        _ => {}
    }

    TRUE
}

fn initialize() -> Result<(), String> {
    try!(logger::init().map_err(|e| format!("Error initializing the logger: {}", e)));
    error!(target: "", "Error");
    warn!(target: "", "Warn");
    info!(target: "", "Info");
    debug!(target: "", "Debug");
    trace!(target: "", "Trace");

    if let Some(kernel32) = ModuleInfo::get("kernel32.dll") {
        unsafe {
            hooks::kernel32::k32.hook(&kernel32, vec![
                &mut hooks::engine::engine,
                &mut hooks::server::server
            ]);
            hooks::kernel32::k32.initial_hook();
        }
    }

    Ok(())
}

fn main_thread() {
    if let Err(err) = initialize() {
        utils::msgbox(&err);
    }
}
