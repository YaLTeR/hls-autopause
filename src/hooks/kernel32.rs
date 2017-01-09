use hookable::Hookable;
use libc;
use moduleinfo::ModuleInfo;
use std::{self, ffi};
use widestring::WideCStr;
use winapi::*;

hook_struct! {
    k32 = pub struct Kernel32 {
        pub module_info: Option<ModuleInfo> = None,
        pub hooks: Option<Vec<&'static mut Hookable>> = None,
    }

    impl Kernel32 {
        pub extern "system" fn LoadLibraryA(&mut self, lpFileName: LPCSTR) -> HMODULE {
            let rv = Kernel32::LoadLibraryA(lpFileName);

            let filename = unsafe { ffi::CStr::from_ptr(lpFileName).to_string_lossy() };
            trace!(target: "kernel32", "LoadLibraryA(\"{}\") -> {:p}", filename, rv);

            rv
        }

        pub extern "system" fn LoadLibraryW(&mut self, lpFileName: LPCWSTR) -> HMODULE {
            let rv = Kernel32::LoadLibraryW(lpFileName);

            let filename = unsafe { WideCStr::from_ptr_str(lpFileName).to_string_lossy() };
            trace!(target: "kernel32", "LoadLibraryW(\"{}\") -> {:p}", filename, rv);

            rv
        }

        pub extern "system" fn LoadLibraryExA(&mut self, lpFileName: LPCSTR, hFile: HANDLE, dwFlags: DWORD) -> HMODULE {
            let rv = Kernel32::LoadLibraryExA(lpFileName, hFile, dwFlags);

            let filename = unsafe { ffi::CStr::from_ptr(lpFileName).to_string_lossy() };
            trace!(target: "kernel32", "LoadLibraryExA(\"{}\") -> {:p}", filename, rv);

            rv
        }

        pub extern "system" fn LoadLibraryExW(&mut self, lpFileName: LPCWSTR, hFile: HANDLE, dwFlags: DWORD) -> HMODULE {
            let rv = Kernel32::LoadLibraryExW(lpFileName, hFile, dwFlags);

            let filename = unsafe { WideCStr::from_ptr_str(lpFileName).to_string_lossy() };
            trace!(target: "kernel32", "LoadLibraryExW(\"{}\") -> {:p}", filename, rv);

            rv
        }

        pub extern "system" fn FreeLibrary(&mut self, hModule: HMODULE) -> BOOL {
            let rv = Kernel32::FreeLibrary(hModule);

            trace!(target: "kernel32", "FreeLibrary({:p}) -> {}", hModule, rv);

            rv
        }
    }
}

impl Kernel32 {
    pub fn hook(&mut self, module_info: &ModuleInfo, hooks: Vec<&'static mut Hookable>) {
        self.module_info = Some(module_info.clone());
        let module_info = self.module_info.as_ref().unwrap();

        self.hooks = Some(hooks);

        debug!(target: "kernel32", "Base: {:p}; size = {}", module_info.base, module_info.size);

        let addr_LoadLibraryA = module_info.get_function(cstr!(b"LoadLibraryA\0"));
        let addr_LoadLibraryW = module_info.get_function(cstr!(b"LoadLibraryW\0"));
        let addr_LoadLibraryExA = module_info.get_function(cstr!(b"LoadLibraryExA\0"));
        let addr_LoadLibraryExW = module_info.get_function(cstr!(b"LoadLibraryExW\0"));
        let addr_FreeLibrary = module_info.get_function(cstr!(b"FreeLibrary\0"));

        print_addrs!("kernel32",
            (addr_LoadLibraryA, "LoadLibraryA"),
            (addr_LoadLibraryW, "LoadLibraryW"),
            (addr_LoadLibraryExA, "LoadLibraryExA"),
            (addr_LoadLibraryExW, "LoadLibraryExW"),
            (addr_FreeLibrary, "FreeLibrary")
        );

        hook!("kernel32", self,
            (addr_LoadLibraryA, LoadLibraryA),
            (addr_LoadLibraryW, LoadLibraryW),
            (addr_LoadLibraryExA, LoadLibraryExA),
            (addr_LoadLibraryExW, LoadLibraryExW),
            (addr_FreeLibrary, FreeLibrary)
        );
    }

    pub fn initial_hook(&mut self) {
        let modules = ModuleInfo::get_loaded();

        for hook in self.hooks.as_mut().unwrap().iter_mut() {
            if let Some(module) = hook.pick_best_hook_target(&modules) {
                hook.hook(module);
            }
        }
    }
}
