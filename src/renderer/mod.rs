use std::mem::ManuallyDrop;

use crate::{sys, Size, ViewId};

#[cfg(feature = "metal")]
mod metal;
#[cfg(feature = "opengl")]
mod opengl;
mod software;
#[cfg(feature = "vulkan")]
mod vulkan;

#[cfg(feature = "metal")]
pub use metal::*;
#[cfg(feature = "opengl")]
pub use opengl::*;
pub use software::*;
#[cfg(feature = "vulkan")]
pub use vulkan::*;

pub struct BackingStoreConfig {
    pub size: Size<f64>,
    pub view_id: ViewId,
}

impl From<sys::FlutterBackingStoreConfig> for BackingStoreConfig {
    fn from(config: sys::FlutterBackingStoreConfig) -> Self {
        Self {
            size: config.size.into(),
            view_id: ViewId(config.view_id),
        }
    }
}

impl From<BackingStoreConfig> for sys::FlutterBackingStoreConfig {
    fn from(config: BackingStoreConfig) -> Self {
        Self {
            struct_size: std::mem::size_of::<Self>(),
            size: config.size.into(),
            view_id: config.view_id.0,
        }
    }
}

// Native type has a did_update field. It's never used in Flutter, so i don't bother with it.
pub enum BackingStore {
    #[cfg(feature = "opengl")]
    OpenGL(OpenGLBackingStore),
    Software(SoftwareBackingStore),
    #[cfg(feature = "metal")]
    Metal(MetalBackingStore),
    #[cfg(feature = "vulkan")]
    Vulkan(VulkanBackingStore),
}

impl From<BackingStore> for sys::FlutterBackingStore {
    fn from(backing_store: BackingStore) -> Self {
        let (type_, __bindgen_anon_1) = match backing_store {
            #[cfg(feature = "opengl")]
            BackingStore::OpenGL(opengl) => (
                sys::FlutterBackingStoreType::OpenGL,
                sys::FlutterBackingStore__bindgen_ty_1 {
                    open_gl: ManuallyDrop::new(opengl.into()),
                },
            ),
            BackingStore::Software(software) => (
                sys::FlutterBackingStoreType::Software2,
                sys::FlutterBackingStore__bindgen_ty_1 {
                    software2: ManuallyDrop::new(software.into()),
                },
            ),
            #[cfg(feature = "metal")]
            BackingStore::Metal(metal) => (
                sys::FlutterBackingStoreType::Metal,
                sys::FlutterBackingStore__bindgen_ty_1 {
                    metal: ManuallyDrop::new(metal.into()),
                },
            ),
            #[cfg(feature = "vulkan")]
            BackingStore::Vulkan(vulkan) => (
                sys::FlutterBackingStoreType::Vulkan,
                sys::FlutterBackingStore__bindgen_ty_1 {
                    vulkan: ManuallyDrop::new(vulkan.into()),
                },
            ),
        };

        Self {
            struct_size: std::mem::size_of::<Self>(),
            user_data: std::ptr::null_mut(),
            did_update: false,
            type_,
            __bindgen_anon_1,
        }
    }
}

impl BackingStore {
    pub(crate) fn from_raw(backing_store: &sys::FlutterBackingStore) -> Self {
        match backing_store.type_ {
            #[cfg(feature = "opengl")]
            sys::FlutterBackingStoreType::OpenGL => {
                BackingStore::OpenGL(OpenGLBackingStore::from_raw(unsafe {
                    &backing_store.__bindgen_anon_1.open_gl
                }))
            }
            #[cfg(not(feature = "opengl"))]
            sys::FlutterBackingStoreType::OpenGL => {
                panic!("OpenGL feature is not enabled. Cannot create the backing store.")
            }
            sys::FlutterBackingStoreType::Software => {
                // unreachable!() because fluster never constructs this type; you have to use underlying sys apis to construct it.
                unreachable!(
                    "Deprecated software backing store type is unsupported. Don't construct it."
                )
            }
            sys::FlutterBackingStoreType::Software2 => {
                BackingStore::Software(SoftwareBackingStore::from_raw(unsafe {
                    &backing_store.__bindgen_anon_1.software2
                }))
            }
            #[cfg(feature = "metal")]
            sys::FlutterBackingStoreType::Metal => {
                BackingStore::Metal(MetalBackingStore::from_raw(unsafe {
                    &backing_store.__bindgen_anon_1.metal
                }))
            }
            #[cfg(not(feature = "metal"))]
            sys::FlutterBackingStoreType::Metal => {
                panic!("Metal feature is not enabled. Cannot create the backing store.")
            }
            #[cfg(feature = "vulkan")]
            sys::FlutterBackingStoreType::Vulkan => {
                BackingStore::Vulkan(VulkanBackingStore::from_raw(unsafe {
                    &backing_store.__bindgen_anon_1.vulkan
                }))
            }
            #[cfg(not(feature = "vulkan"))]
            sys::FlutterBackingStoreType::Vulkan => {
                panic!("Vulkan feature is not enabled. Cannot create the backing store.")
            }
            _ => panic!("Unknown FlutterBackingStoreType. Cannot create the backing store."),
        }
    }
}

pub enum RendererConfig {
    #[cfg(feature = "opengl")]
    OpenGL(OpenGLRendererConfig),
    Software(SoftwareRendererConfig),
    #[cfg(feature = "metal")]
    Metal(MetalRendererConfig),
    #[cfg(feature = "vulkan")]
    Vulkan(VulkanRendererConfig),
}

pub(crate) enum RendererUserData {
    #[cfg(feature = "opengl")]
    OpenGL(OpenGLRendererUserData),
    Software(SoftwareRendererUserData),
    #[cfg(feature = "metal")]
    Metal(MetalRendererUserData),
    #[cfg(feature = "vulkan")]
    Vulkan(VulkanRendererUserData),
}

impl From<RendererConfig> for (RendererUserData, sys::FlutterRendererConfig) {
    fn from(config: RendererConfig) -> Self {
        match config {
            #[cfg(feature = "opengl")]
            RendererConfig::OpenGL(opengl) => {
                let (user_data, config) = opengl.into();
                (
                    RendererUserData::OpenGL(user_data),
                    sys::FlutterRendererConfig {
                        type_: sys::FlutterRendererType::OpenGL,
                        __bindgen_anon_1: sys::FlutterRendererConfig__bindgen_ty_1 {
                            open_gl: config,
                        },
                    },
                )
            }
            RendererConfig::Software(software) => {
                let (user_data, config) = software.into();
                (
                    RendererUserData::Software(user_data),
                    sys::FlutterRendererConfig {
                        type_: sys::FlutterRendererType::Software,
                        __bindgen_anon_1: sys::FlutterRendererConfig__bindgen_ty_1 {
                            software: config,
                        },
                    },
                )
            }
            #[cfg(feature = "metal")]
            RendererConfig::Metal(metal) => {
                let (user_data, config) = metal.into();
                (
                    RendererUserData::Metal(user_data),
                    sys::FlutterRendererConfig {
                        type_: sys::FlutterRendererType::Metal,
                        __bindgen_anon_1: sys::FlutterRendererConfig__bindgen_ty_1 {
                            metal: config,
                        },
                    },
                )
            }
            #[cfg(feature = "vulkan")]
            RendererConfig::Vulkan(vulkan) => {
                let (user_data, config) = vulkan.into();
                (
                    RendererUserData::Vulkan(user_data),
                    sys::FlutterRendererConfig {
                        type_: sys::FlutterRendererType::Vulkan,
                        __bindgen_anon_1: sys::FlutterRendererConfig__bindgen_ty_1 {
                            vulkan: config,
                        },
                    },
                )
            }
        }
    }
}
