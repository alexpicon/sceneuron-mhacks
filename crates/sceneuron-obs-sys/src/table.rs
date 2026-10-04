// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! The macro that declares a table of C function pointers resolved by symbol name.

/// Declares a `Copy` struct whose fields are C function pointers named after the C symbol they
/// hold, plus `SYMBOLS` (every symbol name, in declaration order) and `resolve`, which looks each
/// symbol up and fails with a [`LoadError`](crate::LoadError) naming the first one that is missing.
macro_rules! function_table {
    (
        $(#[$meta:meta])*
        pub struct $table:ident {
            $( $field:ident: $ty:ty, )+
        }
    ) => {
        $(#[$meta])*
        #[derive(Debug, Clone, Copy)]
        pub struct $table {
            $( pub $field: $ty, )+
        }

        impl $table {
            /// The C name of every function in the table, in declaration order.
            pub const SYMBOLS: &'static [&'static ::core::ffi::CStr] = &[$( $crate::table::symbol!($field), )+];

            /// Resolves every function through `lookup`, which returns the address of the named
            /// symbol or null when it is absent.
            ///
            /// A non-null address must be a function with the C signature of the field of the same
            /// name. That is what makes the later `unsafe` calls through the table sound; the
            /// platform lookups used by [`load`](crate::load) satisfy it for a real OBS.
            ///
            /// # Errors
            ///
            /// Returns a [`LoadError`](crate::LoadError) naming the first symbol `lookup` did not
            /// find.
            pub fn resolve(
                lookup: impl Fn(&::core::ffi::CStr) -> *mut ::core::ffi::c_void,
            ) -> Result<Self, $crate::LoadError> {
                Ok(Self {
                    $( $field: {
                        let address = lookup($crate::table::symbol!($field));
                        if address.is_null() {
                            return Err($crate::LoadError::missing(stringify!($field)));
                        }
                        // SAFETY: a data pointer and a function pointer have the same size and
                        // representation on every supported target, and the address is non-null.
                        // That it points to a function of this signature is the documented
                        // obligation of `lookup`; nothing calls it without an `unsafe` block.
                        unsafe { ::core::mem::transmute::<*mut ::core::ffi::c_void, $ty>(address) }
                    }, )+
                })
            }
        }
    };
}

/// The C symbol name for a table field, as a `&'static CStr` built at compile time.
macro_rules! symbol {
    ($field:ident) => {
        const {
            match ::core::ffi::CStr::from_bytes_with_nul(concat!(stringify!($field), "\0").as_bytes()) {
                Ok(name) => name,
                Err(_) => panic!("symbol names contain no NUL byte"),
            }
        }
    };
}

pub(crate) use {function_table, symbol};
