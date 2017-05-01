use features;
use function::Function;
use hookable::*;
use libc::*;
use moduleinfo::ModuleInfo;
use std::sync::RwLock;
use winapi;

lazy_static! {
    pub static ref MODULE: RwLock<ServerModule> = RwLock::new(ServerModule::default());
    static ref DATA: RwLock<Data> = RwLock::new(Data::default());
}

#[derive(Default)]
struct Data {
    jumped_last_tick: bool,
    inside_checkjumpbutton: bool,
}

#[derive(Default)]
pub struct ServerModule {
    module_info: Option<ModuleInfo>,
    current_name_index: Option<usize>,
}

hook_struct! {
    #[derive(Default)]
    pub struct Server {
        pub off_mv: isize,
        pub off_oldbuttons: isize,
    }

    impl Server {
        pub extern "fastcall" fn CHL1GameMovement__CheckJumpButton(this: *mut c_void) {
            let mut server = DATA.write().unwrap();

            const IN_JUMP: c_int = 1 << 1;

            let mut orig_oldbuttons = 0;
            let mut oldbuttons = 0 as *mut c_int;

            if features::autojump() {
                let pointers = POINTERS.read().unwrap();

                let mv = unsafe { *((this as *mut u8).offset(pointers.off_mv) as *mut *mut u8) };
                oldbuttons = unsafe { mv.offset(pointers.off_oldbuttons) as *mut c_int };
                orig_oldbuttons = unsafe { *oldbuttons };

                // If we jumped last tick we can't jump this tick
                // (since this would be the -jump tick).
                if !server.jumped_last_tick {
                    // Make the game think jump wasn't pressed last tick.
                    unsafe {
                        *oldbuttons &= !IN_JUMP;
                    }
                }

                server.jumped_last_tick = false;
            }

            server.inside_checkjumpbutton = true;

            drop(server);
            Server::CHL1GameMovement__CheckJumpButton(this);
            server = DATA.write().unwrap();

            server.inside_checkjumpbutton = false;

            if features::autojump() {
                if !server.jumped_last_tick {
                    // We didn't jump this tick, restore the original jump button state.
                    unsafe {
                        *oldbuttons = orig_oldbuttons;
                    }
                }
            }
        }

        pub extern "fastcall" fn CGameMovement__FinishGravity(this: *mut c_void) {
            if features::autojump() {
                let mut server = DATA.write().unwrap();

                if server.inside_checkjumpbutton {
                    server.jumped_last_tick = true;
                }
            }

            Server::CGameMovement__FinishGravity(this);
        }
    }
}

pattern!(CHL1GameMovement__CheckJumpButton
    0x83 0xEC 0x14 0x53 0x56 0x8B 0xF1 0x57 0x8B 0x7E 0x08 0x85 0xFF 0x74 0x12 0x8B 0x07 0x8B 0xCF 0xFF 0x90 0x60 0x01 0x00 0x00 0x84 0xC0 0x74 0x04 0x8B 0xCF 0xEB
);

pattern!(CGameMovement__FinishGravity
    0x8B 0x51 0x08 0xD9 0x82 0xB0 0x0B 0x00 0x00 0xD8 0x1D ?? ?? ?? ?? 0xDF 0xE0 0xF6 0xC4 0x44 0x7A 0x4D 0xD9 0x82 0x08 0x02 0x00 0x00 0xD8 0x1D
);

impl Hookable for ServerModule {
    fn module_info(&self) -> Option<&ModuleInfo> {
        self.module_info.as_ref()
    }

    fn hook(&mut self, module_info: &ModuleInfo) {
        self.module_info = Some(module_info.clone());

        self.current_name_index = self.compute_current_name_index(module_info);

        debug!(target: "server", "Base: {:p}; size = {}", module_info.base, module_info.size);

        let addr_CHL1GameMovement__CheckJumpButton =
            module_info.find(CHL1GameMovement__CheckJumpButton);
        let addr_CGameMovement__FinishGravity = module_info.find(CGameMovement__FinishGravity);

        print_addrs!("server",
            (addr_CHL1GameMovement__CheckJumpButton, "CHL1GameMovement::CheckJumpButton()"),
            (addr_CGameMovement__FinishGravity, "CGameMovement::FinishGravity()")
        );

        let mut pointers = POINTERS.write().unwrap();

        pointers.off_mv = 4;
        pointers.off_oldbuttons = 40;

        hook!("server", Server, pointers,
            (addr_CHL1GameMovement__CheckJumpButton, CHL1GameMovement__CheckJumpButton),
            (addr_CGameMovement__FinishGravity, CGameMovement__FinishGravity)
        );
    }

    fn unhook(&mut self) {
        let mut pointers = POINTERS.write().unwrap();

        unhook!("server", pointers,
            CHL1GameMovement__CheckJumpButton,
            CGameMovement__FinishGravity
        );

        *DATA.write().unwrap() = Data::default();
        *pointers = Server::default();
        *self = Self::default();
    }
}

impl HookableOrderedNameFilter for ServerModule {
    fn get_current_name_index(&self) -> Option<usize> {
        self.current_name_index
    }

    fn get_names(&self) -> &[&'static str] {
        const NAMES: &'static [&'static str] = &[ "server.dll" ];
        NAMES
    }
}
