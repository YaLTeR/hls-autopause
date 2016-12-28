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

mod hooks;
#[macro_use]
mod minhook;
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

	let addr_Cbuf_AddText = try!(patterns::find(&engine, &patterns::Cbuf_AddText).ok_or("Couldn't find Cbuf_AddText()."));
	let addr_Host_Spawn_f = try!(patterns::find(&engine, &patterns::Host_Spawn_f).ok_or("Couldn't find Host_Spawn_f()."));
	let addr_Host_UnPause_f = try!(patterns::find(&engine, &patterns::Host_UnPause_f).ok_or("Couldn't find Host_UnPause_f()."));
	let addr_CHL1GameMovement__CheckJumpButton = try!(patterns::find(&server, &patterns::CHL1GameMovement__CheckJumpButton).ok_or("Couldn't find CHL1GameMovement::CheckJumpButton()."));

	unsafe {
		hooks::engine.Cbuf_AddText = *(&addr_Cbuf_AddText as *const _ as *const extern "C" fn(*const libc::c_char));

		try!(hook!(addr_Host_Spawn_f, hooks::Engine::Host_Spawn_f_hook, &mut hooks::engine.Host_Spawn_f).map_err(|e| format!("Error creating hook: {}", e)));
		try!(hook!(addr_Host_UnPause_f, hooks::Engine::Host_UnPause_f_hook, &mut hooks::engine.Host_UnPause_f).map_err(|e| format!("Error creating hook: {}", e)));
		try!(hook!(addr_CHL1GameMovement__CheckJumpButton, hooks::Server::CHL1GameMovement__CheckJumpButton_hook, &mut hooks::server.CHL1GameMovement__CheckJumpButton).map_err(|e| format!("Error creating hook: {}", e)));
	}

	try!(minhook::enable_hook(None).map_err(|e| format!("Error enabling hooks: {}", e)));

	Ok(())
}

fn main_thread() {
	if let Err(err) = initialize() {
		utils::msgbox(&err);
	}
}
