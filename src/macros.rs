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
		hook_struct_fields! { $stype $($rest)* F $name { extern $call fn($($arg : $t),*) $(-> $rv)* } }
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
		pub fn $fname($($arg : $t),*) $(-> $rv)* {
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
		pub fn $fname($($arg : $t),*) $(-> $rv)* {
			unsafe {
				($name.$fname)($($arg),*)
			}
		}

		fn [My $fname](&mut $s, $($arg : $t),*) $(-> $rv)* $body

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

macro_rules! pattern {
	($name:ident $(; $byte:tt $mask:expr)* , ?? $($rest:tt)*) => (
		pattern!($name $(; $byte $mask)* ; 0x00 false , $($rest)*);
	);

	($name:ident $(; $byte:tt $mask:expr)* , $b:tt $($rest:tt)*) => (
		pattern!($name $(; $byte $mask)* ; $b true , $($rest)*);
	);

	($name:ident $(; $byte:tt $mask:expr)+ ,) => (
		pub const $name: $crate::pattern::Pattern = $crate::pattern::Pattern(&[$(($byte, $mask)),*]);
	);

	($name:ident $($rest:tt)+) => (
		pattern!($name , $($rest)*);
	);
}

macro_rules! hook {
	($s:ident, $(($target:expr, $fname:ident)),+) => {{
		$(
			try!({ interpolate_idents! {
				let detour = Self::[$fname _hook];
				let trampoline = &mut $s.$fname;

				// This is needed to cast from function item type to function pointer type.
				let mut temp = *trampoline;
				temp = detour;

				$crate::minhook::create_hook($target, temp, trampoline)
					.map_err(|e| format!("Error creating hook: {}", e))
					.and($crate::minhook::queue_enable_hook(Some($target))
						.map_err(|e| format!("Error adding hook to enable queue: {}", e)))
			} });
		)*

		try!($crate::minhook::apply_queued().map_err(|e| format!("Error enabling queued hooks: {}", e)));
	}}
}

macro_rules! cstr {
	($s:expr) => ($s as *const _ as *const libc::c_char)
}

macro_rules! con_command {
	($name:ident, $name_:tt $body:block) => ( interpolate_idents! {
		extern "C" fn [$name _callback]() $body

		static mut $name: $crate::hooks::engine::icvar::ConCommand = $crate::hooks::engine::icvar::ConCommand {
			base:  $crate::hooks::engine::icvar::ConCommandBase {
				vtable: 0 as *mut _,
				next: 0 as *mut _,
				registered: false,
				name: cstr!($name_),
				help_string: 0 as *const _,
				flags: 0,
			},

			callback: [$name _callback],
			completion_callback: $crate::hooks::engine::icvar::ConCommand::default_completion_callback,
			has_completion_callback: true,
		};
	} )
}
