use hooks::engine::Engine;
use libc;
use moduleinfo::ModuleInfo;
use patterns;
use std;

hook_struct! {
	server = pub struct Server {
		pub module_info: Option<ModuleInfo> = None,
	}

	impl Server {
		pub extern "fastcall" fn CHL1GameMovement__CheckJumpButton(&mut self, this: *mut libc::c_void) {
			Engine::Cbuf_AddText(cstr!(b"echo CheckJumpButton\n\0"));

			Server::CHL1GameMovement__CheckJumpButton(this);
		}
	}
}

impl Server {
	pub fn hook(&mut self, module_info: ModuleInfo) -> Result<(), String> {
		self.module_info = Some(module_info);
		let module_info = self.module_info.as_ref().unwrap();

		let addr_CHL1GameMovement__CheckJumpButton = try!(patterns::find(module_info, &patterns::CHL1GameMovement__CheckJumpButton).ok_or("Couldn't find CHL1GameMovement::CheckJumpButton()."));

		try!(hook!(addr_CHL1GameMovement__CheckJumpButton, Server::CHL1GameMovement__CheckJumpButton_hook, &mut self.CHL1GameMovement__CheckJumpButton).map_err(|e| format!("Error creating hook: {}", e)));

		Ok(())
	}
}
