use std::{collections::HashMap, mem::ManuallyDrop};

use tracing::trace;

use crate::{sys, FrameInfo, PresentInfo, Region, Transformation};

pub enum OpenGLBackingStore {
    Texture(OpenGLTexture),
    Framebuffer(OpenGLFramebuffer),
}
impl From<OpenGLBackingStore> for sys::FlutterOpenGLBackingStore {
    fn from(backing_store: OpenGLBackingStore) -> Self {
        match backing_store {
            OpenGLBackingStore::Texture(texture) => Self {
                type_: sys::FlutterOpenGLTargetType::Texture,
                __bindgen_anon_1: sys::FlutterOpenGLBackingStore__bindgen_ty_1 {
                    texture: ManuallyDrop::new(texture.into()),
                },
            },
            OpenGLBackingStore::Framebuffer(framebuffer) => Self {
                type_: sys::FlutterOpenGLTargetType::Framebuffer,
                __bindgen_anon_1: sys::FlutterOpenGLBackingStore__bindgen_ty_1 {
                    framebuffer: ManuallyDrop::new(framebuffer.into()),
                },
            },
        }
    }
}

impl OpenGLBackingStore {
    pub(crate) fn from_raw(raw: &sys::FlutterOpenGLBackingStore) -> Self {
        match raw.type_ {
            sys::FlutterOpenGLTargetType::Texture => {
                OpenGLBackingStore::Texture(OpenGLTexture::from_raw(unsafe {
                    &raw.__bindgen_anon_1.texture
                }))
            }
            sys::FlutterOpenGLTargetType::Framebuffer => {
                OpenGLBackingStore::Framebuffer(OpenGLFramebuffer::from_raw(unsafe {
                    &raw.__bindgen_anon_1.framebuffer
                }))
            }
            _ => unreachable!("Unknown FlutterOpenGLTargetType. Cannot use it."),
        }
    }
}

pub struct OpenGLTexture {
    /// Target texture of the active texture unit (example `GL_TEXTURE_2D` or `GL_TEXTURE_RECTANGLE`).
    pub target: u32,
    /// The name of the texture.
    pub name: u32,
    /// The texture format (example `GL_RGBA8`).
    pub format: u32,
    /// Optional parameters for texture height/width, default is 0, non-zero means
    /// the texture has the specified width/height. Usually, when the texture type
    /// is `GL_TEXTURE_RECTANGLE`, we need to specify the texture width/height to
    /// tell the embedder to scale when rendering.
    /// Width of the texture.
    pub width: usize,
    /// Height of the texture.
    pub height: usize,
}

pub extern "C" fn destroy_opengl_texture_callback(user_data: *mut std::ffi::c_void) {
    let _ = user_data;
    trace!("destroy_opengl_texture_callback");
}
const _: sys::VoidCallback = Some(destroy_opengl_texture_callback);

impl From<OpenGLTexture> for sys::FlutterOpenGLTexture {
    fn from(texture: OpenGLTexture) -> Self {
        Self {
            user_data: std::ptr::null_mut(),
            destruction_callback: Some(destroy_opengl_texture_callback),

            target: texture.target,
            name: texture.name,
            format: texture.format,
            width: texture.width,
            height: texture.height,
        }
    }
}

impl OpenGLTexture {
    fn from_raw(texture: &sys::FlutterOpenGLTexture) -> Self {
        assert!(texture.destruction_callback == Some(destroy_opengl_texture_callback),
         "from_raw(&sys::FlutterOpenGLTexture) for an OpenGL texture for which we didn't set the destruction callback"
        );

        Self {
            target: texture.target,
            name: texture.name,
            format: texture.format,
            width: texture.width,
            height: texture.height,
        }
    }
}

pub struct OpenGLFramebuffer {
    /// The format of the color attachment of the frame-buffer. For example,
    /// GL_RGBA8.
    ///
    /// In case of ambiguity when dealing with Window bound frame-buffers, 0 may
    /// be used.
    pub format: u32,
    /// The name of the framebuffer.
    pub name: u32,
}

extern "C" fn destroy_opengl_framebuffer_callback(user_data: *mut std::ffi::c_void) {
    let _ = user_data;
    trace!("destroy_opengl_framebuffer_callback");
}
const _: sys::VoidCallback = Some(destroy_opengl_framebuffer_callback);

impl From<OpenGLFramebuffer> for sys::FlutterOpenGLFramebuffer {
    fn from(framebuffer: OpenGLFramebuffer) -> Self {
        Self {
            user_data: std::ptr::null_mut(),
            destruction_callback: Some(destroy_opengl_framebuffer_callback),

            // flutter embedder bug: this field is incorrectly named `target` instead of `format`
            target: framebuffer.format,
            name: framebuffer.name,
        }
    }
}
impl OpenGLFramebuffer {
    fn from_raw(raw: &sys::FlutterOpenGLFramebuffer) -> Self {
        assert!(raw.destruction_callback == Some(destroy_opengl_framebuffer_callback),
         "from_raw(&sys::FlutterOpenGLFramebuffer) for an OpenGL framebuffer for which we didn't set the destruction callback"
        );

        Self {
            format: raw.target,
            name: raw.name,
        }
    }
}

pub trait OpenGLRendererHandler {
    fn make_current(&mut self) -> bool;
    fn clear_current(&mut self) -> bool;

    /// The return value indicates success of the present call. This
    /// callback is essential for dirty region management. If not defined, all the
    /// pixels on the screen will be rendered at every frame (regardless of
    /// whether damage is actually being computed or not). This is because the
    /// information that is passed along to the callback contains the frame and
    /// buffer damage that are essential for dirty region management.
    fn present(&mut self, present_info: PresentInfo) -> bool;

    // The return value indicates the id of the frame buffer object (fbo)
    /// that flutter will obtain the gl surface from. The embedder is passed a
    /// `FlutterFrameInfo` struct that indicates the properties of the surface
    /// that flutter will acquire from the returned fbo.
    fn fbo_callback(&mut self, frame_info: FrameInfo) -> u32;

    /// This is an optional callback. Flutter will ask the emebdder to create a GL
    /// context current on a background thread. If the embedder is able to do so,
    /// Flutter will assume that this context is in the same sharegroup as the
    /// main rendering context and use this context for asynchronous texture
    /// uploads. Though optional, it is recommended that all embedders set this
    /// callback as it will lead to better performance in texture handling.
    fn make_resource_current(&mut self) -> bool {
        false
    }

    /// The transformation to apply to the render target before any rendering
    /// operations. This callback is optional.
    /// @attention      When using a custom compositor, the layer offset and sizes
    ///                 will be affected by this transformation. It will be
    ///                 embedder responsibility to render contents at the
    ///                 transformed offset and size. This is useful for embedders
    ///                 that want to render transformed contents directly into
    ///                 hardware overlay planes without having to apply extra
    ///                 transformations to layer contents (which may necessitate
    ///                 an expensive off-screen render pass).
    fn surface_transformation(&mut self) -> Transformation<f64> {
        Transformation::identity()
    }

    fn gl_proc_resolver(&mut self, name: *const std::os::raw::c_char) -> *mut std::ffi::c_void;

    /// When the embedder specifies that a texture has a frame available, the
    /// engine will call this method (on an internal engine managed thread) so
    /// that external texture details can be supplied to the engine for subsequent
    /// composition.
    fn gl_external_texture_frame(
        &mut self,
        texture_id: i64,
        width: usize,
        height: usize,
    ) -> Option<OpenGLTexture>;

    /// Specifying this callback is a requirement for dirty region management.
    /// Dirty region management will only render the areas of the screen that have
    /// changed in between frames, greatly reducing rendering times and energy
    /// consumption. To take advantage of these benefits, it is necessary to
    /// define `populate_existing_damage` as a callback that takes user
    /// data, an FBO ID, and an existing damage [`crate::Region`]. The callback should
    /// use the given FBO ID to identify the FBO's exisiting damage (i.e. areas
    /// that have changed since the FBO was last used) and use it to populate the
    /// given existing damage variable. Not specifying `populate_existing_damage` will result in full
    /// repaint (i.e. rendering all the pixels on the screen at every frame).
    fn populate_existing_damage(&mut self, fbo_id: isize) -> Region;
}

pub struct OpenGLRendererConfig {
    /// By default, the renderer config assumes that the FBO does not change for the duration of the engine run.
    /// If this argument is true, the engine will ask the embedder for
    /// an updated FBO target (via an `fbo_callback` invocation) after a present call.
    pub fbo_reset_after_present: bool,

    /// trait object that contains all the callbacks that the engine will invoke
    pub handler: Box<dyn OpenGLRendererHandler>,
}

impl From<OpenGLRendererConfig> for super::RendererConfig {
    fn from(config: OpenGLRendererConfig) -> Self {
        Self::OpenGL(config)
    }
}

pub(crate) struct OpenGLRendererUserData {
    /// Okay, so this is a fucking hack, lol.
    /// It is not clear to me that how i'm handling this is correct, but it seems to be the intended way.
    ///
    /// We allocate a new damage array (Box<[`sys::FlutterRect`]>)
    /// every time we call `populate_existing_damage`, and store it in the `existing_damage_map`.
    /// This is done at the start of a frame to get the damage region it should repaint.
    /// Example: <https://github.com/flutter/engine/blob/e7c3915fec137d1ba075bdaf07ad643040a0cf41/examples/glfw_drm/FlutterEmbedderGLFW.cc#L230-L232>
    ///
    /// When the engine calls `present_with_info`, the frame is *finished*.
    /// No more repaining will happen, not for this framebuffer object (`fbo_id`) at least.
    /// So, the engine is done with it. We should now drop it.
    /// Example: <https://github.com/flutter/engine/blob/e7c3915fec137d1ba075bdaf07ad643040a0cf41/examples/glfw_drm/FlutterEmbedderGLFW.cc#L162-L166>
    ///
    /// Why doesn't this just have a destruction callback like some other objects?
    existing_damage_map: HashMap<isize, *mut [sys::FlutterRect]>,
    handler: Box<dyn OpenGLRendererHandler>,
}

mod callbacks {
    use crate::{sys, util::return_out_param, EngineUserData, PresentInfo, RendererUserData};

    pub extern "C" fn make_current(engine_user_data: *mut std::ffi::c_void) -> bool {
        let engine_user_data = engine_user_data.cast::<EngineUserData>();
        let engine_user_data = unsafe { &mut *engine_user_data };

        let RendererUserData::OpenGL(user_data) = &mut engine_user_data.renderer_user_data else {
            unreachable!("OpenGL renderer callback called with non-OpenGL renderer user data.");
        };

        user_data.handler.make_current()
    }

    pub extern "C" fn clear_current(engine_user_data: *mut std::ffi::c_void) -> bool {
        let engine_user_data = engine_user_data.cast::<EngineUserData>();
        let engine_user_data = unsafe { &mut *engine_user_data };

        let RendererUserData::OpenGL(user_data) = &mut engine_user_data.renderer_user_data else {
            unreachable!("OpenGL renderer callback called with non-OpenGL renderer user data.");
        };

        user_data.handler.clear_current()
    }

    pub extern "C" fn present_with_info(
        engine_user_data: *mut std::ffi::c_void,
        present_info: *const sys::FlutterPresentInfo,
    ) -> bool {
        let engine_user_data = engine_user_data.cast::<EngineUserData>();
        let engine_user_data = unsafe { &mut *engine_user_data };

        let RendererUserData::OpenGL(user_data) = &mut engine_user_data.renderer_user_data else {
            unreachable!("OpenGL renderer callback called with non-OpenGL renderer user data.");
        };

        let present_info: PresentInfo = PresentInfo::from_raw(unsafe { &*present_info });

        // see field documentation for `existing_damage_map`
        if let Some(existing_damage) = user_data.existing_damage_map.remove(
            &({
                // bro it's flutter's fault for making these inconsistently typed
                #[allow(clippy::cast_possible_wrap)]
                {
                    present_info.fbo_id as isize
                }
            }),
        ) {
            let existing_damage: Box<_> = unsafe { Box::from_raw(existing_damage) };
            drop(existing_damage);
        }

        user_data.handler.present(present_info)
    }

    pub extern "C" fn fbo_with_frame_info(
        engine_user_data: *mut std::ffi::c_void,
        frame_info: *const sys::FlutterFrameInfo,
    ) -> u32 {
        let engine_user_data = engine_user_data.cast::<EngineUserData>();
        let engine_user_data = unsafe { &mut *engine_user_data };

        let RendererUserData::OpenGL(user_data) = &mut engine_user_data.renderer_user_data else {
            unreachable!("OpenGL renderer callback called with non-OpenGL renderer user data.");
        };

        let frame_info = unsafe { *frame_info }.into();

        user_data.handler.fbo_callback(frame_info)
    }

    pub extern "C" fn make_resource_current(engine_user_data: *mut std::ffi::c_void) -> bool {
        let engine_user_data = engine_user_data.cast::<EngineUserData>();
        let engine_user_data = unsafe { &mut *engine_user_data };

        let RendererUserData::OpenGL(user_data) = &mut engine_user_data.renderer_user_data else {
            unreachable!("OpenGL renderer callback called with non-OpenGL renderer user data.");
        };

        user_data.handler.make_resource_current()
    }

    pub extern "C" fn surface_transformation(
        engine_user_data: *mut std::ffi::c_void,
    ) -> sys::FlutterTransformation {
        let engine_user_data = engine_user_data.cast::<EngineUserData>();
        let engine_user_data = unsafe { &mut *engine_user_data };

        let RendererUserData::OpenGL(user_data) = &mut engine_user_data.renderer_user_data else {
            unreachable!("OpenGL renderer callback called with non-OpenGL renderer user data.");
        };

        user_data.handler.surface_transformation().into()
    }

    pub extern "C" fn gl_proc_resolver(
        engine_user_data: *mut std::ffi::c_void,
        name: *const std::os::raw::c_char,
    ) -> *mut std::ffi::c_void {
        let engine_user_data = engine_user_data.cast::<EngineUserData>();
        let engine_user_data = unsafe { &mut *engine_user_data };

        let RendererUserData::OpenGL(user_data) = &mut engine_user_data.renderer_user_data else {
            unreachable!("OpenGL renderer callback called with non-OpenGL renderer user data.");
        };

        user_data.handler.gl_proc_resolver(name)
    }

    pub extern "C" fn gl_external_texture_frame(
        engine_user_data: *mut std::ffi::c_void,
        texture_id: i64,
        width: usize,
        height: usize,
        texture_out: *mut sys::FlutterOpenGLTexture,
    ) -> bool {
        let engine_user_data = engine_user_data.cast::<EngineUserData>();
        let engine_user_data = unsafe { &mut *engine_user_data };

        let RendererUserData::OpenGL(user_data) = &mut engine_user_data.renderer_user_data else {
            unreachable!("OpenGL renderer callback called with non-OpenGL renderer user data.");
        };

        unsafe {
            return_out_param(
                texture_out,
                user_data
                    .handler
                    .gl_external_texture_frame(texture_id, width, height),
            )
        }
    }

    pub extern "C" fn populate_existing_damage(
        engine_user_data: *mut std::ffi::c_void,
        fbo_id: isize,
        existing_damage_out: *mut sys::FlutterDamage,
    ) {
        let engine_user_data = engine_user_data.cast::<EngineUserData>();
        let engine_user_data = unsafe { &mut *engine_user_data };

        let RendererUserData::OpenGL(user_data) = &mut engine_user_data.renderer_user_data else {
            unreachable!("OpenGL renderer callback called with non-OpenGL renderer user data.");
        };

        let existing_damage: Box<[sys::FlutterRect]> = user_data
            .handler
            .populate_existing_damage(fbo_id)
            .regions
            .into_iter()
            .map(Into::into)
            .collect::<Vec<sys::FlutterRect>>()
            .into_boxed_slice();

        let existing_damage = Box::into_raw(existing_damage);

        // see field documentation for `existing_damage_map`
        user_data
            .existing_damage_map
            .insert(fbo_id, existing_damage);

        unsafe {
            existing_damage_out.write(sys::FlutterDamage {
                struct_size: std::mem::size_of::<sys::FlutterDamage>(),
                num_rects: existing_damage.len(),
                damage: existing_damage.cast::<sys::FlutterRect>(),
            });
        }
    }

    const _: sys::BoolCallback = Some(make_current);
    const _: sys::BoolCallback = Some(clear_current);
    const _: sys::BoolPresentInfoCallback = Some(present_with_info);
    const _: sys::UIntFrameInfoCallback = Some(fbo_with_frame_info);
    const _: sys::BoolCallback = Some(make_resource_current);
    const _: sys::TransformationCallback = Some(surface_transformation);
    const _: sys::ProcResolver = Some(gl_proc_resolver);
    const _: sys::TextureFrameCallback = Some(gl_external_texture_frame);
    const _: sys::FlutterFrameBufferWithDamageCallback = Some(populate_existing_damage);
}

impl From<OpenGLRendererConfig> for (OpenGLRendererUserData, sys::FlutterOpenGLRendererConfig) {
    fn from(config: OpenGLRendererConfig) -> Self {
        (
            OpenGLRendererUserData {
                existing_damage_map: HashMap::new(),
                handler: config.handler,
            },
            sys::FlutterOpenGLRendererConfig {
                struct_size: std::mem::size_of::<sys::FlutterOpenGLRendererConfig>(),
                present: None,      // deprecated in favor of present_with_info
                fbo_callback: None, // deprecated in favor of fbo_with_frame_info_callback

                fbo_reset_after_present: config.fbo_reset_after_present,

                make_current: Some(callbacks::make_current),
                clear_current: Some(callbacks::clear_current),
                make_resource_current: Some(callbacks::make_resource_current),
                surface_transformation: Some(callbacks::surface_transformation),
                gl_proc_resolver: Some(callbacks::gl_proc_resolver),
                gl_external_texture_frame_callback: Some(callbacks::gl_external_texture_frame),
                fbo_with_frame_info_callback: Some(callbacks::fbo_with_frame_info),
                present_with_info: Some(callbacks::present_with_info),
                populate_existing_damage: Some(callbacks::populate_existing_damage),
            },
        )
    }
}
