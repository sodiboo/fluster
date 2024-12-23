#![allow(clippy::pedantic, non_snake_case, non_upper_case_globals)]
include!(concat!(env!("OUT_DIR"), "/embedder.rs"));

// the rest of this file is drop glue for the generated types

macro_rules! impl_drop_with_unsafe_destruction_callback {
    ($t:ty) => {
        impl Drop for $t {
            fn drop(&mut self) {
                if let Some(destruction_callback) = self.destruction_callback {
                    unsafe {
                        destruction_callback(self.user_data);
                    }
                }
            }
        }
    };
}

impl_drop_with_unsafe_destruction_callback!(FlutterOpenGLTexture);
impl_drop_with_unsafe_destruction_callback!(FlutterOpenGLFramebuffer);
impl_drop_with_unsafe_destruction_callback!(FlutterSoftwareBackingStore);
impl_drop_with_unsafe_destruction_callback!(FlutterSoftwareBackingStore2);
impl_drop_with_unsafe_destruction_callback!(FlutterMetalTexture);
impl_drop_with_unsafe_destruction_callback!(FlutterVulkanBackingStore);

macro_rules! impl_drop_for_tagged_union {
    ($(
        impl Drop for $t:ty {
            fn drop(&mut self) {
                let union = self.$union:ident;
                unsafe match self.$tag:ident: $tag_type:ty {
                    $(
                        $variant:ident => union.$field:ident,
                    )*
                } else $default:block
            }
        }
    )*) => {
        $(
            impl Drop for $t {
                fn drop(&mut self) {
                    match self.$tag {
                        $(
                            <$tag_type>::$variant => unsafe {
                                ::std::mem::ManuallyDrop::drop(&mut self.$union.$field);
                            },
                        )*
                        _ => $default,
                    }
                }
            }
        )*
    };
}

impl_drop_for_tagged_union! {
    impl Drop for FlutterOpenGLBackingStore {
        fn drop(&mut self) {
            let union = self.__bindgen_anon_1;
            unsafe match self.type_: FlutterOpenGLTargetType {
                Texture => union.texture,
                Framebuffer => union.framebuffer,
            } else {
                panic!("Unknown FlutterOpenGLTargetType. Cannot drop it.");
            }
        }
    }

    impl Drop for FlutterBackingStore {
        fn drop(&mut self) {
            let union = self.__bindgen_anon_1;
            unsafe match self.type_: FlutterBackingStoreType {
                OpenGL => union.open_gl,
                Software => union.software,
                Software2 => union.software2,
                Metal => union.metal,
                Vulkan => union.vulkan,
            } else {
                panic!("Unknown FlutterBackingStoreType. Cannot drop it.");
            }
        }
    }
}

impl Drop for FlutterMetalBackingStore {
    fn drop(&mut self) {
        assert!(
            self.struct_size == std::mem::size_of::<FlutterMetalBackingStore>(),
            "FlutterMetalBackingStore has an unexpected size. It likely has a union tag that i don't know how to handle. It cannot be safely dropped."
        );

        unsafe {
            ::std::mem::ManuallyDrop::drop(&mut self.__bindgen_anon_1.texture);
        }
    }
}
