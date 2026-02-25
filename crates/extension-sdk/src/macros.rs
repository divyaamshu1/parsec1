//! Procedural macros for extension development

#[macro_export]
macro_rules! register_extension {
    ($extension:ty) => {
        #[no_mangle]
        pub extern "C" fn _parsec_extension_create() -> *mut dyn $crate::Extension {
            let extension: $extension = Default::default();
            let boxed: Box<dyn $crate::Extension> = Box::new(extension);
            Box::into_raw(boxed)
        }
        
        #[no_mangle]
        pub extern "C" fn _parsec_extension_destroy(ptr: *mut dyn $crate::Extension) {
            if !ptr.is_null() {
                unsafe { drop(Box::from_raw(ptr)) };
            }
        }
    };
}

#[macro_export]
macro_rules! command {
    ($name:expr, $handler:expr) => {
        // Command registration helper
    };
}