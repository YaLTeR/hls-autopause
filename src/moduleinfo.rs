use kernel32;
use pattern;
use psapi;
use std::{cmp, mem, ptr};
use utils;
use winapi::*;

#[derive(Clone, Copy)]
pub struct ModuleInfo {
    pub handle: HMODULE,
    pub base: LPVOID,
    pub size: usize,
}

unsafe impl Send for ModuleInfo {}
unsafe impl Sync for ModuleInfo {}

impl ModuleInfo {
    pub fn get(name: &str) -> Option<ModuleInfo> {
        unsafe {
            let handle = kernel32::GetModuleHandleW(utils::utf16(name).as_ptr());

            if handle.is_null() {
                return None;
            }

            utils::get_module_info(handle)
        }
    }

    pub fn get_loaded() -> Vec<ModuleInfo> {
        unsafe {
            let mut modules: [HMODULE; 1024] = mem::uninitialized();
            let mut size_needed: DWORD = mem::uninitialized();

            if psapi::EnumProcessModules(kernel32::GetCurrentProcess(),
                                         modules.as_mut_ptr(),
                                         mem::size_of_val(&modules) as DWORD,
                                         &mut size_needed) != 0 {
                let module_count = cmp::min(1024usize,
                                            size_needed as usize / mem::size_of::<HMODULE>());

                modules.into_iter()
                       .cloned()
                       .take(module_count)
                       .filter_map(utils::get_module_info)
                       .collect()
            } else {
                Vec::new()
            }
        }
    }

    pub fn get_function(&self, name: LPCSTR) -> Option<LPVOID> {
        unsafe {
            match kernel32::GetProcAddress(self.handle, name) {
                p if p == ptr::null() => None,
                p => Some(p as LPVOID),
            }
        }
    }

    pub fn find(&self, pattern: pattern::Pattern) -> Option<LPVOID> {
        if self.size < pattern.len() {
            return None;
        }

        let start = self.base as *const u8;
        let end = self.size - pattern.len();

        for i in 0..end {
            let ptr = unsafe { start.offset(i as isize) };

            if pattern.compare(ptr) {
                return Some(ptr as LPVOID);
            }
        }

        None
    }
}
