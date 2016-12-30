use libc;
use libc::*;
use moduleinfo::ModuleInfo;
use patterns;
use std;
use std::mem;
use std::ptr;

pub mod icvar;
use self::icvar::*;

hook_struct! {
	engine = pub struct Engine {
		pub module_info: Option<ModuleInfo> = None,

		pub next_unpause_is_bad: bool = false,
		pub Cbuf_AddText: extern "C" fn(text: *const c_char),
		pub CreateInterface: extern "C" fn(name: *const c_char, return_code: *mut c_int) -> *mut c_void,
		pub icvar: *mut ICVar = 0 as *mut _,
		pub concommand_vtable: *mut c_void = 0 as *mut _,
	}

	impl Engine {
		pub extern "C" fn Host_Spawn_f(&mut self) {
			Engine::Host_Spawn_f();

			self.next_unpause_is_bad = true;
		}

		pub extern "C" fn Host_UnPause_f(&mut self) {
			if self.next_unpause_is_bad {
				self.next_unpause_is_bad = false;
				Engine::Cbuf_AddText(cstr!(b"setpause\n\0"));
			}

			Engine::Host_UnPause_f();
		}
	}
}

con_command!(hello, b"hello\0" {
	Engine::Cbuf_AddText(cstr!(b"echo hello\n\0"));
});

impl Engine {
	pub fn hook(&mut self, module_info: ModuleInfo) -> Result<(), String> {
		self.module_info = Some(module_info);
		let module_info = self.module_info.as_ref().unwrap();

		let addr_Cbuf_AddText = try!(patterns::find(module_info, &patterns::Cbuf_AddText).ok_or("Couldn't find Cbuf_AddText()."));
		let addr_Host_Spawn_f = try!(patterns::find(module_info, &patterns::Host_Spawn_f).ok_or("Couldn't find Host_Spawn_f()."));
		let addr_Host_UnPause_f = try!(patterns::find(module_info, &patterns::Host_UnPause_f).ok_or("Couldn't find Host_UnPause_f()."));
		let addr_ConCommand_constructor = try!(patterns::find(module_info, &patterns::ConCommand_constructor).ok_or("Couldn't find ConCommand::ConCommand()."));
		let addr_CreateInterface = try!(module_info.get_function(cstr!(b"CreateInterface\0")).ok_or("Couldn't get the address of CreateInterface()."));

		unsafe {
			self.Cbuf_AddText = mem::transmute(addr_Cbuf_AddText);
			self.CreateInterface = mem::transmute(addr_CreateInterface);
		}

		self.icvar = try!(self.create_interface(VENGINE_CVAR_INTERFACE_VERSION).ok_or("Couldn't get the ICVar interface from the engine.")) as *mut ICVar;
		self.concommand_vtable = unsafe { *((addr_ConCommand_constructor as *mut u8).offset(35) as *const *mut c_void) };

		unsafe {
			self.register_concmd(&mut hello);
		}

		try!(hook!(addr_Host_Spawn_f, Engine::Host_Spawn_f_hook, &mut self.Host_Spawn_f).map_err(|e| format!("Error creating hook: {}", e)));
		try!(hook!(addr_Host_UnPause_f, Engine::Host_UnPause_f_hook, &mut self.Host_UnPause_f).map_err(|e| format!("Error creating hook: {}", e)));

		Ok(())
	}

	fn create_interface(&self, name: *const c_char) -> Option<*mut c_void> {
		match (self.CreateInterface)(name, ptr::null_mut()) {
			p if p == ptr::null_mut() => None,
			p => Some(p)
		}
	}

	fn register_concmd(&self, concmd: &mut ConCommand) {
		concmd.base.vtable = self.concommand_vtable;

		unsafe {
			(*self.icvar).register_concommandbase(concmd);
		}
	}
}
