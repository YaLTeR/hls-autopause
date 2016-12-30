use kernel32;
use psapi;
use std::mem;
use std::ptr;
use utils;
use winapi::*;

pub struct ModuleInfo {
	pub handle: HMODULE,
	pub base: LPVOID,
	pub size: usize
}

impl ModuleInfo {
	pub fn get(name: &str) -> Option<ModuleInfo> {
		unsafe {
			let handle = kernel32::GetModuleHandleW(utils::utf16(name).as_ptr());

			if !handle.is_null() {
				let mut info = mem::uninitialized::<MODULEINFO>();
				if psapi::GetModuleInformation(kernel32::GetCurrentProcess(), handle, &mut info, mem::size_of::<MODULEINFO>() as DWORD) != 0 {
					return Some(ModuleInfo {
						handle: handle,
						base: info.lpBaseOfDll,
						size: info.SizeOfImage as usize
					});
				}
			}
		}

		None
	}

	pub fn get_function(&self, name: LPCSTR) -> Option<FARPROC> {
		unsafe {
			match kernel32::GetProcAddress(self.handle, name) {
				p if p == ptr::null() => None,
				p => Some(p)
			}
		}
	}
}
