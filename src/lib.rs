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
extern crate psapi;
extern crate user32;
extern crate winapi;

use std::thread;
use winapi::*;

#[macro_use]
mod minhook;

mod hooks;
mod moduleinfo;
use moduleinfo::ModuleInfo;
mod patterns;
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
		},
		DLL_PROCESS_DETACH => {
			minhook::uninitialize();
		}
		_ => {}
	}

	TRUE
}

fn initialize() -> Result<(), String> {
	let engine = try!(ModuleInfo::get("engine.dll").ok_or("Could not get engine.dll module info."));
	let server = try!(ModuleInfo::get("server.dll").ok_or("Could not get server.dll module info."));

	unsafe {
		try!(hooks::engine.hook(engine));
		try!(hooks::server.hook(server));
	}

	try!(minhook::enable_hook(None).map_err(|e| format!("Error enabling hooks: {}", e)));

	Ok(())
}

fn main_thread() {
	if let Err(err) = initialize() {
		utils::msgbox(&err);
	}
}
