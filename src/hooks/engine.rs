use libc;
use libc::*;
use moduleinfo::ModuleInfo;
use patterns;
use std;
use std::ptr;

#[repr(C)]
pub struct ConCommandBase {
	pub vtable: *mut c_void,

	pub next: *mut ConCommandBase,
	pub registered: bool,
	pub name: *const c_char,
	pub help_string: *const c_char,
	pub flags: c_int,
}

type FnCommandCallback = extern "C" fn();
type FnCommandCompletionCallback = extern "C" fn(partial: *const c_char, commands: *const *mut c_char) -> c_int;

#[repr(C)]
pub struct ConCommand {
	pub base: ConCommandBase,

	pub callback: FnCommandCallback,
	pub completion_callback: FnCommandCompletionCallback,
	pub has_completion_callback: bool,
}

impl ConCommand {
	extern "C" fn default_completion_callback(_partial: *const c_char, _commands: *const *mut c_char) -> c_int {
		0
	}
}

unsafe impl Sync for ConCommand {}

// Not declared.
type ConVar = ConCommandBase;

#[repr(C)]
struct ICVarVtable {
	pub RegisterConCommandBase: extern "fastcall" fn(this: *mut ICVar, edx: i32, variable: *mut ConCommandBase) -> *mut c_void,
	pub GetCommandLineValue: extern "fastcall" fn(this: *mut ICVar, edx: i32, variable_name: *const c_char) -> *const c_char,
	pub FindVar: extern "fastcall" fn(this: *mut ICVar, edx: i32, name: *const c_char) -> *const ConVar,
	pub GetCommands: extern "fastcall" fn(this: *mut ICVar) -> *mut ConCommandBase,
}

#[repr(C)]
struct ICVar {
	vtable: *mut ICVarVtable,
}

const VENGINE_CVAR_INTERFACE_VERSION: *const c_char = cstr!(b"VEngineCvar001\0");

hook_struct! {
	engine = pub struct Engine {
		pub module_info: Option<ModuleInfo> = None,

		pub next_unpause_is_bad: bool = false,
		pub Cbuf_AddText: extern "C" fn(text: *const c_char),
		pub CreateInterface: extern "C" fn(name: *const c_char, return_code: *mut c_int) -> *mut c_void,
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

extern "C" fn test_cmd() {
	Engine::Cbuf_AddText(cstr!(b"echo hello\n\0"));
}

static mut test: ConCommand = ConCommand {
	base: ConCommandBase {
		vtable: 0 as *mut _,
		next: 0 as *mut _,
		registered: false,
		name: cstr!(b"hello\0"),
		help_string: 0 as *const _,
		flags: 0,
	},

	callback: test_cmd,
	completion_callback: ConCommand::default_completion_callback,
	has_completion_callback: true,
};

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
			self.Cbuf_AddText = *(&addr_Cbuf_AddText as *const _ as *const extern "C" fn(*const c_char));
			self.CreateInterface = *(&addr_CreateInterface as *const _ as *const extern "C" fn(name: *const c_char, return_code: *mut c_int) -> *mut c_void);
		}

		try!(hook!(addr_Host_Spawn_f, Engine::Host_Spawn_f_hook, &mut self.Host_Spawn_f).map_err(|e| format!("Error creating hook: {}", e)));
		try!(hook!(addr_Host_UnPause_f, Engine::Host_UnPause_f_hook, &mut self.Host_UnPause_f).map_err(|e| format!("Error creating hook: {}", e)));

		let icvar = try!(self.create_interface(VENGINE_CVAR_INTERFACE_VERSION).ok_or("Couldn't get the ICVar interface from the engine.")) as *mut ICVar;
		unsafe {
			let concommand_vtable = *((addr_ConCommand_constructor as *mut u8).offset(35) as *const *mut c_void);
			test.base.vtable = concommand_vtable;

			((*(*icvar).vtable).RegisterConCommandBase)(icvar, 0, &test as *const _ as *mut ConCommandBase);
		}

		Ok(())
	}

	fn create_interface(&self, name: *const c_char) -> Option<*mut c_void> {
		match (self.CreateInterface)(name, ptr::null_mut()) {
			p if p == ptr::null_mut() => None,
			p => Some(p)
		}
	}
}
