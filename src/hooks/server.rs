use features;
use hookable::*;
use libc::*;
use moduleinfo::ModuleInfo;
use std;

hook_struct! {
    server = pub struct Server {
        pub module_info: Option<ModuleInfo> = None,
        pub current_name_index: Option<usize> = None,

        pub jumped_last_tick: bool = false,
        pub inside_checkjumpbutton: bool = false,
        pub off_mv: isize = 4,
        pub off_oldbuttons: isize = 40,
    }

    impl Server {
        pub extern "fastcall" fn CHL1GameMovement__CheckJumpButton(&mut self, this: *mut c_void) {
            const IN_JUMP: c_int = 1 << 1;

            let mut orig_oldbuttons = 0;
            let mut oldbuttons = 0 as *mut c_int;
            
            if features::autojump() {
                let mv = unsafe { *((this as *mut u8).offset(self.off_mv) as *mut *mut u8) };
                oldbuttons = unsafe { mv.offset(self.off_oldbuttons) as *mut c_int };
                orig_oldbuttons = unsafe { *oldbuttons };

                // If we jumped last tick we can't jump this tick
                // (since this would be the -jump tick).
                if !self.jumped_last_tick {
                    // Make the game think jump wasn't pressed last tick.
                    unsafe {
                        *oldbuttons &= !IN_JUMP;
                    }
                }

                self.jumped_last_tick = false;
            }

            self.inside_checkjumpbutton = true;
            Server::CHL1GameMovement__CheckJumpButton(this);
            self.inside_checkjumpbutton = false;

            if features::autojump() {
                if !self.jumped_last_tick {
                    // We didn't jump this tick, restore the original jump button state.
                    unsafe {
                        *oldbuttons = orig_oldbuttons;
                    }
                }
            }
        }

        pub extern "fastcall" fn CGameMovement__FinishGravity(&mut self, this: *mut c_void) {
            if features::autojump() {
                if self.inside_checkjumpbutton {
                    self.jumped_last_tick = true;
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

impl Hookable for Server {
    fn hook(&mut self, module_info: &ModuleInfo) {
        self.module_info = Some(module_info.clone());
        let module_info = self.module_info.as_ref().unwrap();

        self.current_name_index = self.compute_current_name_index(module_info);

        debug!(target: "server", "Base: {:p}; size = {}", module_info.base, module_info.size);

        let addr_CHL1GameMovement__CheckJumpButton =
            module_info.find(CHL1GameMovement__CheckJumpButton);
        let addr_CGameMovement__FinishGravity = module_info.find(CGameMovement__FinishGravity);

        print_addrs!("server",
            (addr_CHL1GameMovement__CheckJumpButton, "CHL1GameMovement::CheckJumpButton()"),
            (addr_CGameMovement__FinishGravity, "CGameMovement::FinishGravity()")
        );

        hook!("server", self,
            (addr_CHL1GameMovement__CheckJumpButton, CHL1GameMovement__CheckJumpButton),
            (addr_CGameMovement__FinishGravity, CGameMovement__FinishGravity)
        );

        features::refresh();
    }
}

impl HookableOrderedNameFilter for Server {
    fn get_current_name_index(&self) -> Option<usize> {
        self.current_name_index
    }

    fn get_names(&self) -> &[&'static str] {
        const NAMES: &'static [&'static str] = &[ "server.dll" ];
        NAMES
    }
}
