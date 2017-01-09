use features;
use hookable::*;
use libc;
use libc::*;
use moduleinfo::ModuleInfo;
use std::{self, mem, ptr};
use winapi;

pub mod icvar;
use self::icvar::*;

hook_struct! {
    engine = pub struct Engine {
        pub module_info: Option<ModuleInfo> = None,
        pub current_name_index: Option<usize> = None,

        pub next_unpause_is_bad: bool = false,
        pub Cbuf_AddText: extern "C" fn(text: *const c_char),
        pub CreateInterface: extern "C" fn(name: *const c_char,
                                           return_code: *mut c_int) -> *mut c_void,
        pub icvar: *mut ICVar = 0 as *mut _,
        pub concommand_vtable: *mut c_void = 0 as *mut _,
    }

    impl Engine {
        pub extern "C" fn Host_Spawn_f(&mut self) {
            Engine::Host_Spawn_f();

            if features::autopause() {
                self.next_unpause_is_bad = true;
            }
        }

        pub extern "C" fn Host_UnPause_f(&mut self) {
            trace!(target: "engine", "Entering Host_UnPause_f()");

            if features::autopause() {
                if self.next_unpause_is_bad {
                    self.next_unpause_is_bad = false;
                    Engine::Cbuf_AddText(cstr!(b"setpause\n\0"));
                }
            }

            Engine::Host_UnPause_f();

            trace!(target: "engine", "Leaving  Host_UnPause_f()");
        }
    }
}

con_command!(hello, b"hello\0" {
    Engine::Cbuf_AddText(cstr!(b"echo hello\n\0"));
});

pattern!(Cbuf_AddText
    0x8B 0x54 0x24 0x04 0x83 0xC9 0xFF 0x57 0x33 0xC0 0x8B 0xFA 0xF2 0xAE 0x8B 0x3D ?? ?? ?? ?? 0xA1 ?? ?? ?? ?? 0xF7 0xD1 0x49 0x03 0xCF 0x3B 0xC8
);

pattern!(Host_Spawn_f
    0xA1 ?? ?? ?? ?? 0x53 0xBB 0x01 0x00 0x00 0x00 0x3B 0xC3 0x56 0x75 0x11 0x68 ?? ?? ?? ?? 0xFF 0x15 ?? ?? ?? ?? 0x83 0xC4 0x04 0x5E 0x5B
);

pattern!(Host_UnPause_f
    0xA0 ?? ?? ?? ?? 0x84 0xC0 0x74 0x59 0x8B 0x0D ?? ?? ?? ?? 0xB8 0x01 0x00 0x00 0x00 0x3B 0xC8 0x75 0x0A 0x50 0xE8
);

pattern!(ConCommand__ConCommand
    0x8B 0x44 0x24 0x08 0x33 0xD2 0x56 0x8B 0xF1 0x89 0x46 0x18 0x8B 0x44 0x24 0x18 0x3B 0xC2 0x88 0x56 0x08 0x89 0x56 0x0C 0x89 0x56 0x10 0x89 0x56 0x14 0x89 0x56 0x04 0xC7 0x06
);

impl Hookable for Engine {
    fn module_info(&self) -> Option<&ModuleInfo> {
        self.module_info.as_ref()
    }

    fn hook(&mut self, module_info: &ModuleInfo) {
        self.module_info = Some(module_info.clone());
        let module_info = self.module_info.as_ref().unwrap();

        self.current_name_index = self.compute_current_name_index(module_info);

        debug!(target: "engine", "Base: {:p}; size = {}", module_info.base, module_info.size);

        let addr_Cbuf_AddText = module_info.find(Cbuf_AddText);
        let addr_Host_Spawn_f = module_info.find(Host_Spawn_f);
        let addr_Host_UnPause_f = module_info.find(Host_UnPause_f);
        let addr_ConCommand__ConCommand = module_info.find(ConCommand__ConCommand);
        let addr_CreateInterface = module_info.get_function(cstr!(b"CreateInterface\0"));

        print_addrs!("engine",
            (addr_Cbuf_AddText, "Cbuf_AddText()"),
            (addr_Host_Spawn_f, "Host_Spawn_f()"),
            (addr_Host_UnPause_f, "Host_UnPause_f()"),
            (addr_ConCommand__ConCommand, "ConCommand::ConCommand()"),
            (addr_CreateInterface, "CreateInterface()")
        );

        if let Some(addr) = addr_Cbuf_AddText {
            self.Cbuf_AddText = unsafe { mem::transmute(addr) };
        }

        if let Some(addr) = addr_CreateInterface {
            self.CreateInterface = unsafe { mem::transmute(addr) };
            self.icvar = self.create_interface(VENGINE_CVAR_INTERFACE_VERSION)
                .unwrap_or(0 as *mut c_void) as *mut ICVar;
        }

        if let Some(addr) = addr_ConCommand__ConCommand {
            self.concommand_vtable = unsafe {
                *((addr as *mut u8).offset(35) as *const *mut c_void)
            };
        }

        hook!("engine", self,
            (addr_Host_Spawn_f, Host_Spawn_f),
            (addr_Host_UnPause_f, Host_UnPause_f)
        );

        features::refresh();

        if features::console_commands() {
            self.register_concmd(unsafe { &mut hello });
        }
    }

    fn unhook(&mut self) {
        unhook!("server", self,
            Host_Spawn_f,
            Host_UnPause_f
        );

        self.clear();

        features::refresh();
    }
}

impl HookableOrderedNameFilter for Engine {
    fn get_current_name_index(&self) -> Option<usize> {
        self.current_name_index
    }

    fn get_names(&self) -> &[&'static str] {
        const NAMES: &'static [&'static str] = &[ "engine.dll" ];
        NAMES
    }
}

impl Engine {
    fn create_interface(&self, name: *const c_char) -> Option<*mut c_void> {
        match (self.CreateInterface)(name, ptr::null_mut()) {
            p if p == ptr::null_mut() => None,
            p => Some(p),
        }
    }

    fn register_concmd(&self, concmd: &mut ConCommand) {
        concmd.base.vtable = self.concommand_vtable;

        unsafe {
            (*self.icvar).register_concommandbase(concmd);
        }
    }
}
