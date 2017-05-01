macro_rules! hook_struct_declare {
    ($stype:ident $(F $fname:ident { $($ftype:tt)* })*) => (
        #[derive(Default)]
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
    ($stype:ident pub $name:ident : $t:ty, $($rest:tt)*) => (
        hook_struct_fields! { $stype $($rest)* F $name { $t } }
    );

    // Function
    ($stype:ident pub extern $call:tt fn $name:ident($($arg:ident : $t:ty),*) $(-> $rv:ty)* { $($body:tt)* } $($rest:tt)*) => (
        hook_struct_fields! { $stype $($rest)* F $name { Function<extern $call fn($($t),*) $(-> $rv)*> } }
    );
}

macro_rules! hook_struct_impl {
    // Function field
    (pub $fname:ident : Function<extern $call:tt fn($($arg:ident : $t:ty),*) $(-> $rv:ty)*> , $($rest:tt)*) => ( interpolate_idents! {
        #[allow(dead_code)]
        #[inline(always)]
        pub fn $fname($($arg : $t),*) $(-> $rv)* {
            let f = POINTERS.read().unwrap().$fname;
            f.call($($arg),*)
        }

        hook_struct_impl! { $($rest)* }
    } );

    // Field
    (pub $fname:ident : $t:ty, $($rest:tt)*) => (
        // We don't care about these here.
        hook_struct_impl! { $($rest)* }
    );

    // Function
    (pub extern $call:tt fn $fname:ident($($arg:ident : $t:ty),*) $(-> $rv:ty)* { $($body:tt)* } $($rest:tt)*) => ( interpolate_idents! {
        #[allow(dead_code)]
        #[inline(always)]
        pub fn $fname($($arg : $t),*) $(-> $rv)* {
            let f = POINTERS.read().unwrap().$fname;
            f.call($($arg),*)
        }

        extern $call fn [My $fname]($($arg : $t),*) $(-> $rv)* {
            $($body)*
        }
    } hook_struct_impl! { $($rest)* } );

    () => ();
}

macro_rules! hook_struct {
    (#[derive(Default)]
     pub struct $stype:ident {
         $($fields:tt)*
     }

     impl $_stype:ident {
         $($fns:tt)*
     }) => (
        lazy_static! {
            pub static ref POINTERS: RwLock<$stype> = RwLock::new($stype::default());
        }

        hook_struct_fields! { $stype $($fields)* $($fns)* }

        impl $stype {
            hook_struct_impl! { $($fields)* $($fns)* }
        }
    );
}

macro_rules! gen_function_impls {
    (@make_impl ($($extern_type:tt)*) ($($arg_name:ident : $arg_type:ident),*)) => (
        impl<R $(, $arg_type)*> Default for Function<$($extern_type)* fn($($arg_type),*) -> R> {
            #[inline(always)]
            fn default() -> Self {
                Function {
                    ptr: Self::default_func as $($extern_type)* fn($($arg_type),*) -> R,
                }
            }
        }

        #[allow(dead_code)]
        impl<R $(, $arg_type)*> Function<$($extern_type)* fn($($arg_type),*) -> R> {
            #[inline(always)]
            pub fn is_default(&self) -> bool {
                self.ptr as *const usize == Self::default_func as *const usize
            }

            #[inline(always)]
            pub fn call(&self $(, $arg_name : $arg_type)*) -> R {
                (self.ptr)($($arg_name),*)
            }

            // This should never be called.
            $($extern_type)* fn default_func($(_: $arg_type),*) -> R {
                unsafe { std::intrinsics::breakpoint(); }
                unreachable!();
            }
        }
    );

    (@gen_impls $($arg_name:ident : $arg_type:ident),*) => (
        gen_function_impls!(@make_impl (                 ) ($($arg_name : $arg_type),*));
        gen_function_impls!(@make_impl (extern "C"       ) ($($arg_name : $arg_type),*));
        gen_function_impls!(@make_impl (extern "system"  ) ($($arg_name : $arg_type),*));
        gen_function_impls!(@make_impl (extern "fastcall") ($($arg_name : $arg_type),*));
    );

    () => (
        gen_function_impls!(@gen_impls);
    );

    ($first_arg_name:ident : $first_arg_type:ident $(, $arg_name:ident : $arg_type:ident)*) => (
        gen_function_impls!(@gen_impls $first_arg_name : $first_arg_type $(, $arg_name : $arg_type)*);
        gen_function_impls!($($arg_name : $arg_type),*);
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
    ($target:tt, $stype:ident, $s:ident, $(($ftarget:expr, $fname:ident)),+) => {{
        $(
            if let Some(ftarget) = $ftarget {
                if let Err(err) = { interpolate_idents! {
                    let detour = $stype::[My $fname];
                    let trampoline = &mut $s.$fname.ptr;

                    // This is needed to cast from function item type to function pointer type.
                    let mut temp = *trampoline;
                    temp = detour;

                    $crate::minhook::create_hook(ftarget, temp, trampoline)
                        .map_err(|e| format!("Error creating hook: {}", e))
                        .and($crate::minhook::queue_enable_hook(Some(ftarget))
                            .map_err(|e| format!("Error adding hook to enable queue: {}", e)))
                } } {
                    error!(target: $target, "{}", err);
                }
            }
        )*

        if let Err(err) = $crate::minhook::apply_queued()
            .map_err(|e| format!("Error enabling queued hooks: {}", e)) {
            error!(target: $target, "{}", err);
        }
    }}
}

macro_rules! unhook {
    ($target:tt, $s:ident, $($fname:ident),+) => {{ interpolate_idents! {
        $(
            if !$s.$fname.is_default() {
                if let Err(err) = {
                    $crate::minhook::queue_disable_hook(Some($s.$fname.ptr as winapi::LPVOID))
                            .map_err(|e| format!("Error adding hook to disable queue: {}", e))
                } {
                    error!(target: $target, "{}", err);
                }
            }
        )*

        if let Err(err) = $crate::minhook::apply_queued()
            .map_err(|e| format!("Error disabling queued hooks: {}", e)) {
            error!(target: $target, "{}", err);
        }

        $(
            if !$s.$fname.is_default() {
                if let Err(err) = {
                    $crate::minhook::remove_hook(Some($s.$fname.ptr as winapi::LPVOID))
                            .map_err(|e| format!("Error removing hook: {}", e))
                } {
                    error!(target: $target, "{}", err);
                }
            }
        )*
    } }}
}

macro_rules! cstr {
    ($s:expr) => ($s as *const _ as *const libc::c_char)
}

macro_rules! con_command {
    ($name:ident, $name_:tt $body:block) => ( interpolate_idents! {
        extern "C" fn [$name _callback]() $body

        static mut $name: $crate::hooks::engine::icvar::ConCommand = $crate::hooks::engine::icvar::ConCommand {
            base: $crate::hooks::engine::icvar::ConCommandBase {
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

macro_rules! print_addrs {
    ($target:tt, $(($addr:expr, $name:tt)),*) => {
        $(
            match $addr {
                Some(addr) => debug!(target: $target, "{:p} - {}", addr, $name),
                None => warn!(target: $target, "<not found> - {}", $name),
            };
        )*
    }
}

macro_rules! define_features {
    ($(($fname:ident, $sname:ident, $text:tt)),*) => ( interpolate_idents! {
        $(
            static mut $sname: Feature = Feature {
                name: $text,
                enabled: false,
            };

            #[inline(always)]
            pub fn $fname() -> bool {
                unsafe { $sname.enabled }
            }
        )*

        static FEATURES: &'static [&'static Feature] = unsafe {
            &[
                $(
                    &$sname
                ),*
            ]
        };
    } )
}
