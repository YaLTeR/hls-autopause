use libc;
use libc::*;
use moduleinfo::ModuleInfo;
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
			trace!(target: "engine", "Entering Host_UnPause_f()");

			if self.next_unpause_is_bad {
				self.next_unpause_is_bad = false;
				Engine::Cbuf_AddText(cstr!(b"setpause\n\0"));
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

impl Engine {
	pub fn hook(&mut self, module_info: ModuleInfo) -> Result<(), String> {
		self.module_info = Some(module_info);
		let module_info = self.module_info.as_ref().unwrap();

		debug!(target: "engine", "Base: {:p}; size = {}", module_info.base, module_info.size);

		let addr_Cbuf_AddText = try!(module_info.find(Cbuf_AddText).ok_or("Couldn't find Cbuf_AddText()."));
		let addr_Host_Spawn_f = try!(module_info.find(Host_Spawn_f).ok_or("Couldn't find Host_Spawn_f()."));
		let addr_Host_UnPause_f = try!(module_info.find(Host_UnPause_f).ok_or("Couldn't find Host_UnPause_f()."));
		let addr_ConCommand__ConCommand = try!(module_info.find(ConCommand__ConCommand).ok_or("Couldn't find ConCommand::ConCommand()."));
		let addr_CreateInterface = try!(module_info.get_function(cstr!(b"CreateInterface\0")).ok_or("Couldn't get the address of CreateInterface()."));

		debug!(target: "engine", "{:p} - Cbuf_AddText()", addr_Cbuf_AddText);
		debug!(target: "engine", "{:p} - Host_Spawn_f()", addr_Host_Spawn_f);
		debug!(target: "engine", "{:p} - Host_UnPause_f()", addr_Host_UnPause_f);
		debug!(target: "engine", "{:p} - ConCommand::ConCommand()", addr_ConCommand__ConCommand);
		debug!(target: "engine", "{:p} - CreateInterface()", addr_CreateInterface);

		unsafe {
			self.Cbuf_AddText = mem::transmute(addr_Cbuf_AddText);
			self.CreateInterface = mem::transmute(addr_CreateInterface);
		}

		self.icvar = try!(self.create_interface(VENGINE_CVAR_INTERFACE_VERSION).ok_or("Couldn't get the ICVar interface from the engine.")) as *mut ICVar;
		self.concommand_vtable = unsafe { *((addr_ConCommand__ConCommand as *mut u8).offset(35) as *const *mut c_void) };

		unsafe {
			self.register_concmd(&mut hello);
		}

		hook!(self,
			(addr_Host_Spawn_f, Host_Spawn_f),
			(addr_Host_UnPause_f, Host_UnPause_f)
		);

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
