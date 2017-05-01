use features;
use function::Function;
use hookable::*;
use libc;
use libc::*;
use moduleinfo::ModuleInfo;
use std::{mem, ptr};
use std::sync::RwLock;
use winapi;

pub mod icvar;
use self::icvar::*;

lazy_static! {
    pub static ref MODULE: RwLock<EngineModule> = RwLock::new(EngineModule::default());
    static ref DATA: RwLock<Data> = RwLock::new(Data::default());
}

#[derive(Default)]
struct Data {
    next_unpause_is_bad: bool,
}

#[derive(Default)]
pub struct EngineModule {
    module_info: Option<ModuleInfo>,
    current_name_index: Option<usize>,
}

unsafe impl Send for Engine {}
unsafe impl Sync for Engine {}

hook_struct! {
    #[derive(Default)]
    pub struct Engine {
        pub initialized: bool,

        pub Cbuf_AddText: Function<extern "C" fn(text: *const c_char)>,
        pub CreateInterface: Function<extern "C" fn(name: *const c_char,
                                                    return_code: *mut c_int) -> *mut c_void>,
        pub icvar: Option<*mut ICVar>,
        pub concommand_vtable: Option<*mut c_void>,
    }

    impl Engine {
        pub extern "C" fn Host_Spawn_f() {
            Engine::initialize(); // TODO: this should be somewhere in Host_Frame or Cbuf_Execute.

            Engine::Host_Spawn_f();

            if features::autopause() {
                DATA.write().unwrap().next_unpause_is_bad = true;
            }
        }

        pub extern "C" fn Host_UnPause_f() {
            trace!(target: "engine", "Entering Host_UnPause_f()");

            if features::autopause() {
                let mut engine = DATA.write().unwrap();
                if engine.next_unpause_is_bad {
                    engine.next_unpause_is_bad = false;
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

impl Hookable for EngineModule {
    fn module_info(&self) -> Option<&ModuleInfo> {
        self.module_info.as_ref()
    }

    fn hook(&mut self, module_info: &ModuleInfo) {
        self.module_info = Some(module_info.clone());

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

        let mut pointers = POINTERS.write().unwrap();

        if let Some(addr) = addr_Cbuf_AddText {
            pointers.Cbuf_AddText.ptr = unsafe { mem::transmute(addr) };
        }

        if let Some(addr) = addr_CreateInterface {
            pointers.CreateInterface.ptr = unsafe { mem::transmute(addr) };
        }

        if let Some(addr) = addr_ConCommand__ConCommand {
            pointers.concommand_vtable = Some(unsafe {
                *((addr as *mut u8).offset(35) as *const *mut c_void)
            });
        }

        hook!("engine", Engine, pointers,
            (addr_Host_Spawn_f, Host_Spawn_f),
            (addr_Host_UnPause_f, Host_UnPause_f)
        );
    }

    fn unhook(&mut self) {
        let mut pointers = POINTERS.write().unwrap();

        unhook!("server", pointers,
            Host_Spawn_f,
            Host_UnPause_f
        );

        *DATA.write().unwrap() = Data::default();
        *pointers = Engine::default();
        *self = Self::default();
    }
}

impl HookableOrderedNameFilter for EngineModule {
    fn get_current_name_index(&self) -> Option<usize> {
        self.current_name_index
    }

    fn get_names(&self) -> &[&'static str] {
        const NAMES: &'static [&'static str] = &[ "engine.dll" ];
        NAMES
    }
}

impl Engine {
    // TODO: all this is pretty terrible.
    fn initialize() {
        if POINTERS.read().unwrap().initialized {
            return;
        }

        POINTERS.write().unwrap().initialized = true;

        let icvar = Engine::create_interface(VENGINE_CVAR_INTERFACE_VERSION)
            .map(|p| p as *mut ICVar);
        POINTERS.write().unwrap().icvar = icvar;

        features::refresh();

        if features::console_commands() {
            unsafe { Engine::register_concmd(&mut hello); }
        }
    }

    fn create_interface(name: *const c_char) -> Option<*mut c_void> {
        match Engine::CreateInterface(name, ptr::null_mut()) {
            p if p == ptr::null_mut() => None,
            p => Some(p),
        }
    }

    fn register_concmd(concmd: &mut ConCommand) {
        concmd.base.vtable = POINTERS.read().unwrap().concommand_vtable.unwrap();
        let icvar = POINTERS.read().unwrap().icvar.unwrap();

        unsafe {
            (*icvar).register_concommandbase(concmd);
        }
    }
}
