use libc::*;
use moduleinfo::ModuleInfo;
use patterns;
use std;

hook_struct! {
	server = pub struct Server {
		pub module_info: Option<ModuleInfo> = None,

		pub jumped_last_tick: bool = false,
		pub inside_checkjumpbutton: bool = false,
		pub off_mv: isize = 4,
		pub off_oldbuttons: isize = 40,
	}

	impl Server {
		pub extern "fastcall" fn CHL1GameMovement__CheckJumpButton(&mut self, this: *mut c_void) {
			const IN_JUMP: c_int = 1 << 1;

			let mv = unsafe { *((this as *mut u8).offset(self.off_mv) as *mut *mut u8) };
			let oldbuttons = unsafe { mv.offset(self.off_oldbuttons) as *mut c_int };
			let orig_oldbuttons = unsafe { *oldbuttons };

			// If we jumped last tick we can't jump this tick (since this would be the -jump tick).
			if !self.jumped_last_tick {
				unsafe {
					*oldbuttons &= !IN_JUMP; // Make the game think jump wasn't pressed last tick.
				}
			}

			self.jumped_last_tick = false;

			self.inside_checkjumpbutton = true;
			Server::CHL1GameMovement__CheckJumpButton(this);
			self.inside_checkjumpbutton = false;

			if !self.jumped_last_tick {
				// We didn't jump this tick, restore the original jump button state.
				unsafe {
					*oldbuttons = orig_oldbuttons;
				}
			}
		}
		
		pub extern "fastcall" fn CGameMovement__FinishGravity(&mut self, this: *mut c_void) {
			if self.inside_checkjumpbutton {
				self.jumped_last_tick = true;
			}

			Server::CGameMovement__FinishGravity(this);
		}
	}
}

impl Server {
	pub fn hook(&mut self, module_info: ModuleInfo) -> Result<(), String> {
		self.module_info = Some(module_info);
		let module_info = self.module_info.as_ref().unwrap();

		let addr_CHL1GameMovement__CheckJumpButton = try!(module_info.find(&patterns::CHL1GameMovement__CheckJumpButton).ok_or("Couldn't find CHL1GameMovement::CheckJumpButton()."));
		let addr_CGameMovement__FinishGravity = try!(module_info.find(&patterns::CGameMovement__FinishGravity).ok_or("Couldn't find CGameMovement::FinishGravity()."));

		try!(hook!(addr_CHL1GameMovement__CheckJumpButton, Server::CHL1GameMovement__CheckJumpButton_hook, &mut self.CHL1GameMovement__CheckJumpButton).map_err(|e| format!("Error creating hook: {}", e)));
		try!(hook!(addr_CGameMovement__FinishGravity, Server::CGameMovement__FinishGravity_hook, &mut self.CGameMovement__FinishGravity).map_err(|e| format!("Error creating hook: {}", e)));

		Ok(())
	}
}
