use std::mem::ManuallyDrop;

use metal::foreign_types::{ForeignType, ForeignTypeRef};

use crate::{sys, EngineUserData, FrameInfo};

pub enum MetalBackingStore {
    Texture(MetalTexture),
}
impl From<MetalBackingStore> for sys::FlutterMetalBackingStore {
    fn from(backing_store: MetalBackingStore) -> Self {
        match backing_store {
            MetalBackingStore::Texture(texture) => Self {
                struct_size: std::mem::size_of::<Self>(),
                // In the future, this might have a union tag. Currently it doesn't, there is just one variant.
                // If a future version of the engine with a union tag ever receives this value,
                // it should know how to handle it based on the struct_size we provide to it.
                __bindgen_anon_1: sys::FlutterMetalBackingStore__bindgen_ty_1 {
                    texture: ManuallyDrop::new(texture.into()),
                },
            },
        }
    }
}

impl MetalBackingStore {
    pub(crate) fn from_raw(raw: &sys::FlutterMetalBackingStore) -> Self {
        if raw.struct_size != std::mem::size_of::<MetalBackingStore>() {
            panic!("FlutterMetalBackingStore has an unexpected size. It likely has a union tag that i don't know how to handle. It cannot be safely used.");
        }
        MetalBackingStore::Texture(MetalTexture::from_raw(unsafe {
            &raw.__bindgen_anon_1.texture
        }))
    }
}

#[derive(Clone)]
pub struct MetalTexture {
    /// Embedder provided unique identifier to the texture buffer. Given that the
    /// `texture` handle is passed to the engine to render to, the texture buffer
    /// is itself owned by the embedder. This `texture_id` is then also given to
    /// the embedder in the present callback.
    texture_id: i64,
    /// Handle to the MTLTexture that is owned by the embedder. Engine will render
    /// the frame into this texture.
    //
    // A NULL texture is considered invalid. (this type can't represent NULL)
    texture: metal::Texture,
}

pub extern "C" fn destroy_metal_texture_callback(user_data: *mut std::ffi::c_void) {
    let mtl_texture = user_data as *mut metal::MTLTexture;
    let mtl_texture = unsafe { metal::Texture::from_ptr(mtl_texture) };

    drop(mtl_texture);
}
const _: sys::VoidCallback = Some(destroy_metal_texture_callback);

impl From<MetalTexture> for sys::FlutterMetalTexture {
    fn from(texture: MetalTexture) -> Self {
        // This transfers ownership of MTLTexture from MetalTexture to the new FlutterMetalTexture.

        // into_ptr forgets the previous owner, so it manually gets dropped in the destruction callback.
        let mtl_texture: *mut metal::MTLTexture = texture.texture.into_ptr();

        Self {
            struct_size: std::mem::size_of::<Self>(),
            user_data: mtl_texture as *mut std::ffi::c_void,
            destruction_callback: Some(destroy_metal_texture_callback),

            texture_id: texture.texture_id,
            // FlutterMetalTextureHandle represents *mut MTLTexture.
            texture: mtl_texture as sys::FlutterMetalTextureHandle,
        }
    }
}
impl MetalTexture {
    fn from_raw(raw: &sys::FlutterMetalTexture) -> Self {
        assert!(raw.destruction_callback == Some(destroy_metal_texture_callback),
         "from_raw(&sys::FlutterMetalTexture) called with a metal texture for which we didn't set the destruction callback"
        );

        // This doesn't increment the refcount; runtime thinks only sys::FlutterMetalTexture owns the texture.
        let mtl_texture_ref =
            unsafe { metal::TextureRef::from_ptr(raw.texture as *mut metal::MTLTexture) };

        // But now we increment the refcount, so that this MetalTexture owns the texture.
        let mtl_texture = mtl_texture_ref.to_owned();

        // (yes, i know those statements are obvious, but i'm writing them for clarity
        // because working with objc is unintuitive to me and i want to make sure it's correct)

        Self {
            texture_id: raw.texture_id,
            texture: mtl_texture,
        }
    }
}

simple_enum! {
    pub enum FlutterMetalExternalTexturePixelFormat<sys::FlutterMetalExternalTexturePixelFormat> {
        YUVA = kYUVA,
        RGBA = kRGBA,
    }

    pub enum FlutterMetalExternalTextureYUVColorSpace<sys::FlutterMetalExternalTextureYUVColorSpace> {
        BT601FullRange = kBT601FullRange,
        BT601LimitedRange = kBT601LimitedRange,
    }
}

pub struct MetalExternalTexture {
    width: usize,
    height: usize,
    pixel_format: FlutterMetalExternalTexturePixelFormat,
    yuv_color_space: FlutterMetalExternalTextureYUVColorSpace,
    textures: Vec<sys::FlutterMetalTextureHandle>,
}

// TODO: handle lifetime of FlutterMetalExternalTexture* textures
// maybe like in OpenGL?
impl From<MetalExternalTexture> for sys::FlutterMetalExternalTexture {
    fn from(texture: MetalExternalTexture) -> Self {
        Self {
            struct_size: std::mem::size_of::<Self>(),
            width: texture.width,
            height: texture.height,
            pixel_format: texture.pixel_format.into(),
            num_textures: todo!(),
            textures: todo!(),
            yuv_color_space: todo!(),
        }
    }
}

pub trait MetalRendererHandler {
    /// The callback that gets invoked when the engine requests the embedder for a
    /// texture to render to.
    ///
    /// Not used if a FlutterCompositor is supplied in FlutterProjectArgs.
    fn get_next_drawable(&mut self, frame_info: FrameInfo) -> MetalTexture;
    /// The callback presented to the embedder to present a fully populated metal
    /// texture to the user.
    ///
    /// Not used if a FlutterCompositor is supplied in FlutterProjectArgs.
    fn present_drawable(&mut self, texture: MetalTexture) -> bool;
    /// When the embedder specifies that a texture has a frame available, the
    /// engine will call this method (on an internal engine managed thread) so
    /// that external texture details can be supplied to the engine for subsequent
    /// composition.
    fn external_texture_frame(
        &mut self,
        texture_id: i64,
        width: usize,
        height: usize,
    ) -> Option<MetalExternalTexture>;
}

pub struct MetalRendererConfig {
    pub device: sys::FlutterMetalDeviceHandle,
    pub present_command_queue: sys::FlutterMetalCommandQueueHandle,
    pub handler: Box<dyn MetalRendererHandler>,
}

impl From<MetalRendererConfig> for super::RendererConfig {
    fn from(config: MetalRendererConfig) -> Self {
        Self::Metal(config)
    }
}

pub(crate) struct MetalRendererUserData {
    handler: Box<dyn MetalRendererHandler>,
}

mod callbacks {
    use crate::RendererUserData;

    use super::*;

    pub extern "C" fn get_next_drawable(
        engine_user_data: *mut std::ffi::c_void,
        texture: *const sys::FlutterFrameInfo,
    ) -> sys::FlutterMetalTexture {
        let engine_user_data = engine_user_data as *mut EngineUserData;
        let engine_user_data = unsafe { &mut *engine_user_data };

        let RendererUserData::Metal(user_data) = &mut engine_user_data.renderer_user_data else {
            unreachable!("Metal renderer callback called with non-metal renderer user data.");
        };

        let frame_info = FrameInfo::from(unsafe { *texture });

        user_data.handler.get_next_drawable(frame_info).into()
    }

    pub extern "C" fn present_drawable(
        engine_user_data: *mut std::ffi::c_void,
        texture: *const sys::FlutterMetalTexture,
    ) -> bool {
        let engine_user_data = engine_user_data as *mut EngineUserData;
        let engine_user_data = unsafe { &mut *engine_user_data };

        let RendererUserData::Metal(user_data) = &mut engine_user_data.renderer_user_data else {
            unreachable!("Metal renderer callback called with non-metal renderer user data.");
        };

        let texture = MetalTexture::from_raw(unsafe { &*texture });

        user_data.handler.present_drawable(texture)
    }

    pub extern "C" fn external_texture_frame(
        engine_user_data: *mut std::ffi::c_void,
        texture_id: i64,
        width: usize,
        height: usize,
        texture_out: *mut sys::FlutterMetalExternalTexture,
    ) -> bool {
        let engine_user_data = engine_user_data as *mut EngineUserData;
        let engine_user_data = unsafe { &mut *engine_user_data };

        let RendererUserData::Metal(user_data) = &mut engine_user_data.renderer_user_data else {
            unreachable!("Metal renderer callback called with non-metal renderer user data.");
        };

        let texture = user_data
            .handler
            .external_texture_frame(texture_id, width, height);

        unsafe { crate::util::return_out_param(texture_out, texture) }
    }

    const _: sys::FlutterMetalTextureCallback = Some(get_next_drawable);
    const _: sys::FlutterMetalPresentCallback = Some(present_drawable);
    const _: sys::FlutterMetalTextureFrameCallback = Some(external_texture_frame);
}

impl From<MetalRendererConfig> for (MetalRendererUserData, sys::FlutterMetalRendererConfig) {
    fn from(metal: MetalRendererConfig) -> Self {
        (
            MetalRendererUserData {
                handler: metal.handler,
            },
            sys::FlutterMetalRendererConfig {
                struct_size: std::mem::size_of::<sys::FlutterMetalRendererConfig>(),
                device: metal.device,
                present_command_queue: metal.present_command_queue,
                get_next_drawable_callback: Some(callbacks::get_next_drawable),
                present_drawable_callback: Some(callbacks::present_drawable),
                external_texture_frame_callback: Some(callbacks::external_texture_frame),
            },
        )
    }
}
