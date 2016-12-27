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
pub struct MinHookError {
	status: MH_STATUS,
	description: String,
}

impl fmt::Display for MinHookError {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		write!(f, "{}", &self.description)
	}
}

impl error::Error for MinHookError {
	fn description(&self) -> &str {
		&self.description
	}
}

impl MinHookError {
	fn new(status: MH_STATUS) -> Self {
		MinHookError { status: status, description: status_to_string(status) }
	}
}

#[derive(Debug)]
pub enum Error<'a> {
	InitError(&'a MinHookError),
	OperationError(MinHookError),
}

impl<'a> fmt::Display for Error<'a> {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		match *self {
			Error::InitError(_) => {
				write!(f, "MinHook initialization error: {}", (self as &error::Error).description())
			},
			Error::OperationError(_) => {
				write!(f, "{}", (self as &error::Error).description())
			},
		}
	}
}

impl<'a> error::Error for Error<'a> {
	fn description(&self) -> &str {
		match *self {
			Error::InitError(e) => &e.description,
			Error::OperationError(ref e) => &e.description,
		}
	}

	fn cause(&self) -> Option<&error::Error> {
		match *self {
			Error::InitError(e) => Some(e),
			Error::OperationError(ref e) => Some(e),
		}
	}
}

pub type Result<'a, T> = result::Result<T, Error<'a>>;

lazy_static! {
	static ref mh_init_result: result::Result<(), MinHookError> = unsafe {
		match MH_Initialize() {
			MH_OK => Ok(()),
			err => Err(MinHookError::new(err))
		}
	};
}

pub fn uninitialize() {
	if mh_init_result.is_ok() {
		unsafe { MH_Uninitialize(); }
	}
}

pub fn create_hook<F: Copy>(target: LPVOID, detour: F, trampoline: &mut F) -> Result<'static, ()> {
	if let Err(ref err) = *mh_init_result {
		return Err(Error::InitError(err));
	}

	unsafe {
		let temp = *trampoline;
		*trampoline = detour;
		let detour = *(trampoline as *const _ as *const LPVOID);

		match MH_CreateHook(target, detour, trampoline as *mut _ as *mut LPVOID) {
			MH_OK => Ok(()),
			err => {
				*trampoline = temp;
				Err(Error::OperationError(MinHookError::new(err)))
			}
		}
	}
}

pub fn enable_hook(target: Option<LPVOID>) -> Result<'static, ()> {
	if let Err(ref err) = *mh_init_result {
		return Err(Error::InitError(err));
	}

	let target = target.unwrap_or(ptr::null_mut());

	unsafe {
		match MH_EnableHook(target) {
			MH_OK => Ok(()),
			err => Err(Error::OperationError(MinHookError::new(err)))
		}
	}
}

macro_rules! hook {
	($target:expr, $detour:expr, $trampoline:expr) => {{
		// This is needed to cast from function item type to function pointer type.
		let mut temp = *$trampoline;
		temp = $detour;

		$crate::minhook::create_hook($target, temp, $trampoline)
	}}
}
