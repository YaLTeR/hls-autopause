use libc;
use moduleinfo::ModuleInfo;
use patterns;
use std;
use std::ffi::CString;

macro_rules! hook_struct_declare {
	($stype:ident $(F $fname:ident { $($ftype:tt)* })*) => (
		pub struct $stype {
			$(pub $fname : $($ftype)*),*
		}
	);
}

macro_rules! hook_struct_fields {
	($stype:ident F $($rest:tt)*) => (
		hook_struct_declare! { $stype F $($rest)* }
	);

	// Field
	($stype:ident pub $name:ident : $t:ty = $init:expr , $($rest:tt)*) => (
		hook_struct_fields! { $stype $($rest)* F $name { $t } }
	);

	// Function field
	($stype:ident pub $name:ident : extern $call:tt fn($($arg:ident : $t:ty),*) $(-> $rv:ty)* , $($rest:tt)*) => (
		hook_struct_fields! { $stype $($rest)* F $name { extern $call fn($($arg : $t),*) $(-> $rv:ty)* } }
	);

	// Function
	($stype:ident pub extern $call:tt fn $name:ident(&mut $s:ident $(, $arg:ident : $t:ty)*) $(-> $rv:ty)* $body:block $($rest:tt)*) => (
		hook_struct_fields! { $stype $($rest)* F $name { extern $call fn($(arg : $t),*) $(-> $rv)* } }
	);
}

macro_rules! hook_struct_impl {
	// Field
	($name:ident pub $fname:ident : $t:ty = $init:expr , $($rest:tt)*) => (
		// We don't care about these here.
		hook_struct_impl! { $name $($rest)* }
	);

	// Function field
	($name:ident pub $fname:ident : extern $call:tt fn($($arg:ident : $t:ty),*) $(-> $rv:ty)* , $($rest:tt)*) => ( interpolate_idents! {
		extern $call fn [$fname _default]($(_ : $t),*) $(-> $rv)* {
			// This should never be called.
			unsafe {
				std::intrinsics::breakpoint();
			}

			unreachable!();
		}

		#[allow(dead_code)]
		#[inline(always)]
		pub extern $call fn $fname($($arg : $t),*) $(-> $rv)* {
			unsafe {
				($name.$fname)($($arg),*)
			}
		}

		hook_struct_impl! { $name $($rest)* }
	} );

	// Function
	($name:ident pub extern $call:tt fn $fname:ident(&mut $s:ident $(, $arg:ident : $t:ty)*) $(-> $rv:ty)* $body:block $($rest:tt)*) => ( interpolate_idents! {
		extern $call fn [$fname _default]($(_ : $t),*) $(-> $rv)* {
			// This should never be called.
			unsafe {
				std::intrinsics::breakpoint();
			}

			unreachable!();
		}

		pub extern $call fn [$fname _hook]($($arg : $t),*) $(-> $rv)* {
			unsafe {
				$name.[My $fname]($($arg),*)
			}
		}

		#[allow(dead_code)]
		#[inline(always)]
		pub extern $call fn $fname($($arg : $t),*) $(-> $rv)* {
			unsafe {
				($name.$fname)($($arg),*)
			}
		}

		extern $call fn [My $fname](&mut $s, $($arg : $t),*) $(-> $rv)* $body

	} hook_struct_impl! { $name $($rest)* } );

	($name:ident) => ();
}

macro_rules! hook_struct_gen_init {
	($stype:ident $(I $fname:ident $init:expr)*) => (
		$stype {
			$($fname : $init),*
		}
	);
}

macro_rules! hook_struct_init {
	($stype:ident I $($rest:tt)*) => (
		hook_struct_gen_init! { $stype I $($rest)* }
	);

	// Field
	($stype:ident pub $name:ident : $t:ty = $init:expr , $($rest:tt)*) => (
		hook_struct_init! { $stype $($rest)* I $name $init }
	);

	// Function field
	($stype:ident pub $name:ident : extern $call:tt fn($($arg:ident : $t:ty),*) $(-> $rv:ty)* , $($rest:tt)*) => ( interpolate_idents! {
		hook_struct_init! { $stype $($rest)* I $name $stype :: [$name _default] }
	} );

	// Function
	($stype:ident pub extern $call:tt fn $name:ident(&mut $s:ident $(, $arg:ident : $t:ty)*) $(-> $rv:ty)* $body:block $($rest:tt)*) => ( interpolate_idents! {
		hook_struct_init! { $stype $($rest)* I $name $stype :: [$name _default] }
	} );
}

macro_rules! hook_struct {
	($name:ident = pub struct $stype:ident { $($fields:tt)* } impl $_stype:ident { $($fns:tt)* }) => (
		hook_struct_fields! { $stype $($fields)* $($fns)* }

		impl $stype {
			hook_struct_impl! { $name $($fields)* $($fns)* }
		}

		pub static mut $name: $stype = hook_struct_init! { $stype $($fields)* $($fns)* };
	);
}

hook_struct! {
	engine = pub struct Engine {
		pub module_info: Option<ModuleInfo> = None,

		pub next_unpause_is_bad: bool = false,
		pub Cbuf_AddText: extern "C" fn(text: *const libc::c_char),
	}

	impl Engine {
		pub extern "C" fn Host_Spawn_f(&mut self) {
			Engine::Host_Spawn_f();

			self.next_unpause_is_bad = true;
		}

		pub extern "C" fn Host_UnPause_f(&mut self) {
			if self.next_unpause_is_bad {
				self.next_unpause_is_bad = false;
				Engine::Cbuf_AddText(CString::new("setpause\n").unwrap().as_ptr());
			}

			Engine::Host_UnPause_f();
		}
	}
}

impl Engine {
	pub fn hook(&mut self, module_info: ModuleInfo) -> Result<(), String> {
		self.module_info = Some(module_info);
		let module_info = self.module_info.as_ref().unwrap();

		let addr_Cbuf_AddText = try!(patterns::find(module_info, &patterns::Cbuf_AddText).ok_or("Couldn't find Cbuf_AddText()."));
		let addr_Host_Spawn_f = try!(patterns::find(module_info, &patterns::Host_Spawn_f).ok_or("Couldn't find Host_Spawn_f()."));
		let addr_Host_UnPause_f = try!(patterns::find(module_info, &patterns::Host_UnPause_f).ok_or("Couldn't find Host_UnPause_f()."));

		unsafe {
			self.Cbuf_AddText = *(&addr_Cbuf_AddText as *const _ as *const extern "C" fn(*const libc::c_char));
		}

		try!(hook!(addr_Host_Spawn_f, Engine::Host_Spawn_f_hook, &mut self.Host_Spawn_f).map_err(|e| format!("Error creating hook: {}", e)));
		try!(hook!(addr_Host_UnPause_f, Engine::Host_UnPause_f_hook, &mut self.Host_UnPause_f).map_err(|e| format!("Error creating hook: {}", e)));

		Ok(())
	}
}

hook_struct! {
	server = pub struct Server {
		pub module_info: Option<ModuleInfo> = None,
	}

	impl Server {
		pub extern "fastcall" fn CHL1GameMovement__CheckJumpButton(&mut self, this: *mut libc::c_void) {
			Engine::Cbuf_AddText(CString::new("echo CheckJumpButton\n").unwrap().as_ptr());

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
