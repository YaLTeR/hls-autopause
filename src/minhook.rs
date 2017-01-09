extern crate libc;
extern crate winapi;
extern crate user32;

use std::collections::HashMap;
use std::{error, fmt, ptr, result};
use std::ffi::CStr;
use std::sync::RwLock;
use winapi::*;

#[allow(non_camel_case_types)]
type MH_STATUS = i32;
const MH_OK: MH_STATUS = 0;

#[link(name = "MinHook", kind = "static")]
extern "system" {
    fn MH_Initialize() -> MH_STATUS;
    fn MH_Uninitialize() -> MH_STATUS;
    fn MH_CreateHook(pTarget: LPVOID, pDetour: LPVOID, ppOriginal: *mut LPVOID) -> MH_STATUS;
    fn MH_RemoveHook(pTarget: LPVOID) -> MH_STATUS;
    fn MH_QueueEnableHook(pTarget: LPVOID) -> MH_STATUS;
    fn MH_QueueDisableHook(pTarget: LPVOID) -> MH_STATUS;
    fn MH_ApplyQueued() -> MH_STATUS;
    fn MH_StatusToString(status: MH_STATUS) -> *const libc::c_char;
}

fn status_to_string(status: MH_STATUS) -> String {
    unsafe { CStr::from_ptr(MH_StatusToString(status)).to_string_lossy().into_owned() }
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
        MinHookError {
            status: status,
            description: status_to_string(status),
        }
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
            }
            Error::OperationError(_) => write!(f, "{}", (self as &error::Error).description()),
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

    static ref trampoline_to_target: RwLock<HashMap<usize, usize>> = RwLock::new(HashMap::new());
}

pub fn uninitialize() {
    if mh_init_result.is_ok() {
        unsafe {
            MH_Uninitialize();
        }
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
            MH_OK => {
                trampoline_to_target.write()
                                    .unwrap()
                                    .insert(*(trampoline as *const _ as *const usize),
                                            target as usize);

                Ok(())
            },
            err => {
                *trampoline = temp;
                Err(Error::OperationError(MinHookError::new(err)))
            }
        }
    }
}

pub fn remove_hook(trampoline: Option<LPVOID>) -> Result<'static, ()> {
    if let Err(ref err) = *mh_init_result {
        return Err(Error::InitError(err));
    }

    let target = match trampoline {
        Some(addr) => {
            trampoline_to_target.write()
                                .unwrap()
                                .remove(&(addr as usize))
                                .map(|t| t as LPVOID)
                                .unwrap_or(ptr::null_mut())
        },
        None => ptr::null_mut()
    };

    unsafe {
        match MH_RemoveHook(target) {
            MH_OK => Ok(()),
            err => Err(Error::OperationError(MinHookError::new(err))),
        }
    }
}

pub fn queue_enable_hook(target: Option<LPVOID>) -> Result<'static, ()> {
    if let Err(ref err) = *mh_init_result {
        return Err(Error::InitError(err));
    }

    let target = target.unwrap_or(ptr::null_mut());

    unsafe {
        match MH_QueueEnableHook(target) {
            MH_OK => Ok(()),
            err => Err(Error::OperationError(MinHookError::new(err))),
        }
    }
}

pub fn queue_disable_hook(trampoline: Option<LPVOID>) -> Result<'static, ()> {
    if let Err(ref err) = *mh_init_result {
        return Err(Error::InitError(err));
    }

    let target = match trampoline {
        Some(addr) => {
            trampoline_to_target.read()
                                .unwrap()
                                .get(&(addr as usize))
                                .map(|&t| t as LPVOID)
                                .unwrap_or(ptr::null_mut())
        },
        None => ptr::null_mut()
    };

    unsafe {
        match MH_QueueDisableHook(target) {
            MH_OK => Ok(()),
            err => Err(Error::OperationError(MinHookError::new(err))),
        }
    }
}

pub fn apply_queued() -> Result<'static, ()> {
    if let Err(ref err) = *mh_init_result {
        return Err(Error::InitError(err));
    }

    unsafe {
        match MH_ApplyQueued() {
            MH_OK => Ok(()),
            err => Err(Error::OperationError(MinHookError::new(err))),
        }
    }
}
