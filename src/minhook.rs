extern crate libc;
extern crate winapi;
extern crate user32;

use std::error;
use std::ffi::CStr;
use std::fmt;
use std::ptr;
use std::result;
use winapi::*;

#[allow(non_camel_case_types)]
type MH_STATUS = i32;
const MH_OK: MH_STATUS = 0;

#[link(name = "MinHook", kind = "static")]
extern "system" {
	fn MH_Initialize() -> MH_STATUS;
	fn MH_Uninitialize() -> MH_STATUS;
	fn MH_CreateHook(pTarget: LPVOID, pDetour: LPVOID, ppOriginal: *mut LPVOID) -> MH_STATUS;
	fn MH_EnableHook(pTarget: LPVOID) -> MH_STATUS;
	fn MH_StatusToString(status: MH_STATUS) -> *const libc::c_char;
}

fn status_to_string(status: MH_STATUS) -> String {
	unsafe {
		CStr::from_ptr(MH_StatusToString(status)).to_string_lossy().into_owned()
	}
}

#[derive(Debug)]
pub struct Error {
	status: MH_STATUS,
	description: String,
}

impl fmt::Display for Error {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		write!(f, "{}", &self.description)
	}
}

impl error::Error for Error {
	fn description(&self) -> &str {
		&self.description
	}
}

impl Error {
	fn new(status: MH_STATUS) -> Self {
		Error { status: status, description: status_to_string(status) }
	}
}

pub type Result<T> = result::Result<T, Error>;

pub struct MinHook {
	_h: ()
}

impl MinHook {
	pub fn new() -> Result<Self> {
		unsafe {
			match MH_Initialize() {
				MH_OK => Ok(MinHook { _h: () }),
				err => Err(Error::new(err))
			}
		}
	}

	pub fn create_hook<F: Copy>(&self, target: LPVOID, detour: F, trampoline: &mut F) -> Result<()> {
		unsafe {
			let temp = *trampoline;
			*trampoline = detour;
			let detour = *(trampoline as *const _ as *const LPVOID);

			match MH_CreateHook(target, detour, trampoline as *mut _ as *mut LPVOID) {
				MH_OK => Ok(()),
				err => {
					*trampoline = temp;
					Err(Error::new(err))
				}
			}
		}
	}

	pub fn enable_hook(&self, target: Option<LPVOID>) -> Result<()> {
		unsafe {
			let target = target.unwrap_or(ptr::null_mut());

			match MH_EnableHook(target) {
				MH_OK => Ok(()),
				err => Err(Error::new(err))
			}
		}
	}
}

impl Drop for MinHook {
	fn drop(&mut self) {
		unsafe {
			MH_Uninitialize();
		}
	}
}

macro_rules! hook {
	($mh:expr, $target:expr, $detour:expr, $trampoline:expr) => {{
		// This is needed to cast from function item type to function pointer type.
		let mut temp = *$trampoline;
		temp = $detour;

		$mh.create_hook($target, temp, $trampoline)
	}}
}
