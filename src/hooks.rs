use libc;
use std;
use std::ffi::CString;

macro_rules! define_hooks {
	(pub extern $call:tt fn $name:ident($($arg:ident : $t:ty),*) $(-> $rv:ty)* $body:block $($rest:tt)*) => ( interpolate_idents! {
		extern $call fn [$name _default]($(_: $t),*) $(-> $rv:ty)* {
			// This should never be called.
			unsafe {
				std::intrinsics::breakpoint();
			}

			unreachable!();
		}

		pub static mut $name: extern $call fn($($arg: $t),*) $(-> $rv:ty)* = [$name _default];

		pub extern $call fn [My $name]($($arg: $t),*) $(-> $rv:ty)* $body
	} define_hooks! { $($rest)* } );
	() => ()
}

macro_rules! define_game_functions {
	(pub static mut $name:ident: extern $call:tt fn($($arg:ident : $t:ty),*) $(-> $rv:ty)*; $($rest:tt)*) => ( interpolate_idents! {
		extern $call fn [$name _default]($(_: $t),*) $(-> $rv:ty)* {
			// This should never be called.
			unsafe {
				std::intrinsics::breakpoint();
			}

			unreachable!();
		}

		pub static mut $name: extern $call fn($($arg: $t),*) $(-> $rv:ty)* = [$name _default];
	} define_game_functions! { $($rest)* } );
	() => ()
}

static mut next_unpause_is_bad: bool = false;

define_hooks! {
	pub extern "C" fn Host_Spawn_f() {
		unsafe {
			Host_Spawn_f();

			next_unpause_is_bad = true;
		}
	}

	pub extern "C" fn Host_UnPause_f() {
		unsafe {
			if next_unpause_is_bad {
				next_unpause_is_bad = false;
				Cbuf_AddText(CString::new("setpause\n").unwrap().as_ptr());
			}

			Host_UnPause_f();
		}
	}

	pub extern "fastcall" fn CHL1GameMovement__CheckJumpButton(this: *mut libc::c_void) {
		unsafe {
			Cbuf_AddText(CString::new("echo CheckJumpButton\n").unwrap().as_ptr());

			CHL1GameMovement__CheckJumpButton(this);
		}
	}
}

define_game_functions! {
	pub static mut Cbuf_AddText: extern "C" fn(text: *const libc::c_char);
}
