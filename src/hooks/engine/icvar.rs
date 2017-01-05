use libc;
use libc::*;
use std::ops;

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
type FnCommandCompletionCallback = extern "C" fn(partial: *const c_char,
                                                 commands: *const *mut c_char)
                                                 -> c_int;

#[repr(C)]
pub struct ConCommand {
    pub base: ConCommandBase,

    pub callback: FnCommandCallback,
    pub completion_callback: FnCommandCompletionCallback,
    pub has_completion_callback: bool,
}

impl ops::Deref for ConCommand {
    type Target = ConCommandBase;

    fn deref(&self) -> &ConCommandBase {
        unsafe { &*(self as *const _ as *const ConCommandBase) }
    }
}

impl ops::DerefMut for ConCommand {
    fn deref_mut(&mut self) -> &mut ConCommandBase {
        unsafe { &mut *(self as *mut _ as *mut ConCommandBase) }
    }
}

impl ConCommand {
    pub extern "C" fn default_completion_callback(_partial: *const c_char,
                                                  _commands: *const *mut c_char)
                                                  -> c_int {
        0
    }
}

// Not declared.
type ConVar = ConCommandBase;

#[repr(C)]
struct ICVarVtable {
    pub RegisterConCommandBase: extern "fastcall" fn(this: *mut ICVar,
                                                     edx: i32,
                                                     variable: *mut ConCommandBase),
    pub GetCommandLineValue: extern "fastcall" fn(this: *mut ICVar,
                                                  edx: i32,
                                                  variable_name: *const c_char)
                                                  -> *const c_char,
    pub FindVar: extern "fastcall" fn(this: *mut ICVar, edx: i32, name: *const c_char)
                                      -> *const ConVar,
    pub GetCommands: extern "fastcall" fn(this: *mut ICVar) -> *mut ConCommandBase,
}

#[repr(C)]
pub struct ICVar {
    vtable: *mut ICVarVtable,
}

impl ICVar {
    pub fn register_concommandbase(&mut self, concommandbase: &mut ConCommandBase) {
        unsafe { ((*self.vtable).RegisterConCommandBase)(self, 0, concommandbase) };
    }
}

pub const VENGINE_CVAR_INTERFACE_VERSION: *const c_char = cstr!(b"VEngineCvar001\0");
