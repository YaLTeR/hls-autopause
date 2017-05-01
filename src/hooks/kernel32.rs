use features;
use function::Function;
use hookable::Hookable;
use hooks::*;
use libc;
use moduleinfo::ModuleInfo;
use std::ffi;
use std::sync::RwLock;
use utils;
use widestring::WideCStr;
use winapi::*;

lazy_static! {
    pub static ref MODULE: RwLock<Kernel32Module> = RwLock::new(Kernel32Module::default());

    static ref HOOKS: [&'static RwLock<Hookable>; 2] = [
        server::MODULE.deref(),
        engine::MODULE.deref()
    ];
}

#[derive(Default)]
pub struct Kernel32Module {
    module_info: Option<ModuleInfo>,
}

hook_struct! {
    #[derive(Default)]
    pub struct Kernel32 {
    }

    impl Kernel32 {
        pub extern "system" fn LoadLibraryA(lpFileName: LPCSTR) -> HMODULE {
            let rv = Kernel32::LoadLibraryA(lpFileName);

            let filename = unsafe { ffi::CStr::from_ptr(lpFileName).to_string_lossy() };
            trace!(target: "kernel32", "LoadLibraryA(\"{}\") -> {:p}", filename, rv);

            Kernel32::hook_module(rv);

            rv
        }

        pub extern "system" fn LoadLibraryW(lpFileName: LPCWSTR) -> HMODULE {
            let rv = Kernel32::LoadLibraryW(lpFileName);

            let filename = unsafe { WideCStr::from_ptr_str(lpFileName).to_string_lossy() };
            trace!(target: "kernel32", "LoadLibraryW(\"{}\") -> {:p}", filename, rv);

            Kernel32::hook_module(rv);

            rv
        }

        pub extern "system" fn LoadLibraryExA(lpFileName: LPCSTR, hFile: HANDLE, dwFlags: DWORD) -> HMODULE {
            let rv = Kernel32::LoadLibraryExA(lpFileName, hFile, dwFlags);

            let filename = unsafe { ffi::CStr::from_ptr(lpFileName).to_string_lossy() };
            trace!(target: "kernel32", "LoadLibraryExA(\"{}\") -> {:p}", filename, rv);

            Kernel32::hook_module(rv);

            rv
        }

        pub extern "system" fn LoadLibraryExW(lpFileName: LPCWSTR, hFile: HANDLE, dwFlags: DWORD) -> HMODULE {
            let rv = Kernel32::LoadLibraryExW(lpFileName, hFile, dwFlags);

            let filename = unsafe { WideCStr::from_ptr_str(lpFileName).to_string_lossy() };
            trace!(target: "kernel32", "LoadLibraryExW(\"{}\") -> {:p}", filename, rv);

            Kernel32::hook_module(rv);

            rv
        }

        pub extern "system" fn FreeLibrary(hModule: HMODULE) -> BOOL {
            Kernel32::unhook_module(hModule);

            let rv = Kernel32::FreeLibrary(hModule);

            trace!(target: "kernel32", "FreeLibrary({:p}) -> {}", hModule, rv);

            rv
        }
    }
}

impl Kernel32Module {
    pub fn hook(&mut self, module_info: &ModuleInfo) {
        self.module_info = Some(module_info.clone());

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

        let mut pointers = POINTERS.write().unwrap();

        hook!("kernel32", Kernel32, pointers,
            (addr_LoadLibraryA, LoadLibraryA),
            (addr_LoadLibraryW, LoadLibraryW),
            (addr_LoadLibraryExA, LoadLibraryExA),
            (addr_LoadLibraryExW, LoadLibraryExW),
            (addr_FreeLibrary, FreeLibrary)
        );
    }
}

impl Kernel32 {
    pub fn initial_hook() {
        let modules = ModuleInfo::get_loaded();

        let mut hooked_something = false;

        for hook in HOOKS.iter() {
            if let Some(module) = hook.read().unwrap().pick_best_hook_target(&modules) {
                hook.write().unwrap().hook(module);
                hooked_something = true;
            }
        }

        if hooked_something {
            features::refresh();
        }
    }

    fn hook_module(handle: HMODULE) {
        if let Some(module) = utils::get_module_info(handle) {
            let mut hooked_something = false;

            for hook in HOOKS.iter() {
                if hook.read().unwrap().should_hook(&module) {
                    let mut hook = hook.write().unwrap();

                    if hook.module_info().is_some() {
                        hook.unhook();
                    }

                    hook.hook(&module);

                    hooked_something = true;
                }
            }

            if hooked_something {
                features::refresh();
            }
        }
    }

    fn unhook_module(handle: HMODULE) {
        let mut unhooked_something = false;

        for hook in HOOKS.iter() {
            let hook_read = hook.read().unwrap();

            if hook_read.module_info().is_some() {
                if hook_read.module_info().unwrap().handle == handle {
                    drop(hook_read);
                    hook.write().unwrap().unhook();
                    unhooked_something = true;
                }
            }
        }

        if unhooked_something {
            features::refresh();
        }
    }
}
