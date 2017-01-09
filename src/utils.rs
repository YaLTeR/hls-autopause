use kernel32;
use moduleinfo::ModuleInfo;
use psapi;
use std::{mem, path, ptr};
use user32;
use utils;
use widestring::WideCStr;
use winapi::*;

pub fn utf16(string: &str) -> Vec<u16> {
    string.encode_utf16().chain(Some(0)).collect()
}

pub fn msgbox(message: &str) {
    unsafe {
        user32::MessageBoxW(ptr::null_mut(),
                            utils::utf16(&message).as_ptr(),
                            utils::utf16("HL:S OOE Autopause").as_ptr(),
                            MB_ICONERROR);
    }
}

pub fn get_module_info(handle: HMODULE) -> Option<ModuleInfo> {
    unsafe {
        let mut info = mem::uninitialized::<MODULEINFO>();

        if psapi::GetModuleInformation(kernel32::GetCurrentProcess(),
                                       handle,
                                       &mut info,
                                       mem::size_of::<MODULEINFO>() as DWORD) != 0 {
            Some(ModuleInfo {
                handle: handle,
                base: info.lpBaseOfDll,
                size: info.SizeOfImage as usize,
            })
        } else {
            None
        }
    }
}

pub fn get_module_path(handle: HMODULE) -> Option<path::PathBuf> {
    unsafe {
        let mut filename: [WCHAR; MAX_PATH] = mem::uninitialized();

        let len = kernel32::GetModuleFileNameW(handle, filename.as_mut_ptr(), MAX_PATH as DWORD);
        if len > 0 {
            Some(path::PathBuf::from(
                WideCStr::from_ptr_with_nul(filename.as_ptr(), len as usize).to_os_string()
            ))
        } else {
            None
        }
    }
}
