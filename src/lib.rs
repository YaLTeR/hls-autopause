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

use std::mem::{size_of, uninitialized};
use std::ptr::null_mut;
use std::thread;
use winapi::{
	BOOL,
	DWORD,
	HINSTANCE,
	LPVOID,
	MODULEINFO,
	MB_ICONERROR,
	TRUE
};

mod hooks;
#[macro_use]
mod minhook;
mod patterns;
use patterns::find_pattern;

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

pub struct ModuleInfo {
	base: LPVOID,
	size: usize
}

fn utf16(string: &str) -> Vec<u16> {
	string.encode_utf16().chain(Some(0)).collect()
}

fn msgbox(message: &str) {
	unsafe {
		user32::MessageBoxW(null_mut(), utf16(&message).as_ptr(), utf16("HL:S OOE Autopause").as_ptr(), MB_ICONERROR);
	}
}

fn get_module_info(name: &str) -> Option<ModuleInfo> {
	unsafe {
		let handle = kernel32::GetModuleHandleW(utf16(name).as_ptr());
		if !handle.is_null() {
			let mut info = uninitialized::<MODULEINFO>();
			if psapi::GetModuleInformation(kernel32::GetCurrentProcess(), handle, &mut info, size_of::<MODULEINFO>() as DWORD) != 0 {
				return Some(ModuleInfo { base: info.lpBaseOfDll, size: info.SizeOfImage as usize });
			}
		}
	}

	None
}

fn initialize() -> Result<(), String> {
	let engine = try!(get_module_info("engine.dll").ok_or("Could not get engine.dll module info."));
	let server = try!(get_module_info("server.dll").ok_or("Could not get server.dll module info."));

	let addr_Cbuf_AddText = try!(find_pattern(&engine, &patterns::Cbuf_AddText).ok_or("Couldn't find Cbuf_AddText()."));
	let addr_Host_Spawn_f = try!(find_pattern(&engine, &patterns::Host_Spawn_f).ok_or("Couldn't find Host_Spawn_f()."));
	let addr_Host_UnPause_f = try!(find_pattern(&engine, &patterns::Host_UnPause_f).ok_or("Couldn't find Host_UnPause_f()."));
	let addr_CHL1GameMovement__CheckJumpButton = try!(find_pattern(&server, &patterns::CHL1GameMovement__CheckJumpButton).ok_or("Couldn't find CHL1GameMovement::CheckJumpButton()."));

	unsafe {
		hooks::Cbuf_AddText = *(&addr_Cbuf_AddText as *const _ as *const extern "C" fn(*const libc::c_char));

		try!(hook!(addr_Host_Spawn_f, hooks::MyHost_Spawn_f, &mut hooks::Host_Spawn_f).map_err(|e| format!("Error creating hook: {}", e)));
		try!(hook!(addr_Host_UnPause_f, hooks::MyHost_UnPause_f, &mut hooks::Host_UnPause_f).map_err(|e| format!("Error creating hook: {}", e)));
		try!(hook!(addr_CHL1GameMovement__CheckJumpButton, hooks::MyCHL1GameMovement__CheckJumpButton, &mut hooks::CHL1GameMovement__CheckJumpButton).map_err(|e| format!("Error creating hook: {}", e)));
	}

	try!(minhook::enable_hook(None).map_err(|e| format!("Error enabling hooks: {}", e)));

	Ok(())
}

fn main_thread() {
	if let Err(err) = initialize() {
		msgbox(&err);
	}
}
