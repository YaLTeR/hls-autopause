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

use std::{ptr, thread};
use winapi::*;

#[macro_use]
mod macros;

mod features;
mod function;
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

fn tell_injector_to_resume_process() {
    const EVENT_MODIFY_STATE: DWORD = 0x2;
    let event_name = utils::utf16("BunnymodXT-Injector");

    let event = unsafe { kernel32::OpenEventW(EVENT_MODIFY_STATE, FALSE, event_name.as_ptr()) };
    if event != ptr::null_mut() {
        unsafe {
            kernel32::SetEvent(event);
            kernel32::CloseHandle(event);
        }

        debug!(target: "", "Signaled the injector to resume the process.");
    }
}

fn initialize() -> Result<(), String> {
    try!(logger::init().map_err(|e| format!("Error initializing the logger: {}", e)));
    error!(target: "", "Error");
    warn!(target: "", "Warn");
    info!(target: "", "Info");
    debug!(target: "", "Debug");
    trace!(target: "", "Trace");

    if let Some(kernel32) = ModuleInfo::get("kernel32.dll") {
        hooks::kernel32::MODULE.write().unwrap().hook(&kernel32);
        hooks::kernel32::Kernel32::initial_hook();
    }

    tell_injector_to_resume_process();

    Ok(())
}

fn main_thread() {
    if let Err(err) = initialize() {
        utils::msgbox(&err);
    }
}
