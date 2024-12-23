use std::ffi::{CStr, CString};

use tracing::trace;

use crate::{sys, FrameInfo};

pub struct VulkanImage {
    /// Handle to the `VkImage` that is owned by the embedder. The engine will
    /// bind this image for writing the frame.
    pub image_handle: sys::FlutterVulkanImageHandle,
    /// The `VkFormat` of the image (for example: `VK_FORMAT_R8G8B8A8_UNORM`).
    pub format: u32,
}
impl From<VulkanImage> for sys::FlutterVulkanImage {
    fn from(image: VulkanImage) -> Self {
        Self {
            struct_size: std::mem::size_of::<Self>(),
            image: image.image_handle,
            format: image.format,
        }
    }
}

impl From<sys::FlutterVulkanImage> for VulkanImage {
    fn from(image: sys::FlutterVulkanImage) -> Self {
        Self {
            image_handle: image.image,
            format: image.format,
        }
    }
}

pub struct VulkanBackingStore {
    /// The image that the layer will be rendered to. This image must already be
    /// available for the engine to bind for writing when it's given to the engine
    /// via the backing store creation callback. The engine will perform a host
    /// sync for all layers prior to calling the compositor present callback, and
    /// so the written layer images can be freely bound by the embedder without
    /// any additional synchronization.
    pub image: VulkanImage,
}

extern "C" fn destroy_vulkan_callback(user_data: *mut std::ffi::c_void) {
    trace!("destroy_vulkan_callback");

    let vulkan_image = unsafe { Box::from_raw(user_data.cast::<sys::FlutterVulkanImage>()) };
    drop(vulkan_image);
}
const _: sys::VoidCallback = Some(destroy_vulkan_callback);

impl From<VulkanBackingStore> for sys::FlutterVulkanBackingStore {
    fn from(vulkan: VulkanBackingStore) -> Self {
        let image: Box<sys::FlutterVulkanImage> = Box::new(vulkan.image.into());

        let image = Box::into_raw(image);

        Self {
            struct_size: std::mem::size_of::<Self>(),
            user_data: image.cast::<std::ffi::c_void>(),
            destruction_callback: Some(destroy_vulkan_callback),

            image,
        }
    }
}
impl VulkanBackingStore {
    pub(crate) fn from_raw(raw: &sys::FlutterVulkanBackingStore) -> Self {
        assert!(
            raw.destruction_callback == Some(destroy_vulkan_callback),
            "from_raw(&sys::FlutterVulkanBackingStore) for a vulkan buffer for which we didn't set the destruction callback"
        );
        Self {
            image: VulkanImage::from(unsafe { *raw.image }),
        }
    }
}

pub trait VulkanRendererHandler {
    #[allow(clippy::doc_markdown)] // ugh rewrite it later
    /// The callback invoked when resolving Vulkan function pointers.
    /// At a bare minimum this should be used to swap out any calls that operate
    /// on vkQueue's for threadsafe variants that obtain locks for their duration.
    /// The functions to swap out are "vkQueueSubmit" and "vkQueueWaitIdle".  An
    /// example of how to do that can be found in the test
    /// "EmbedderTest.CanSwapOutVulkanCalls" unit-test in
    /// //shell/platform/embedder/tests/embedder_vk_unittests.cc.
    fn get_instance_proc_address(
        &mut self,
        instance: sys::FlutterVulkanInstanceHandle,
        name: &CStr,
    ) -> *mut std::ffi::c_void;
    /// The callback invoked when the engine requests a `VkImage` from the embedder
    /// for rendering the next frame.
    /// Not used if a `FlutterCompositor` is supplied in `FlutterProjectArgs`.
    fn get_next_image(&mut self, frame_info: FrameInfo) -> VulkanImage;
    /// The callback invoked when a `VkImage` has been written to and is ready for
    /// use by the embedder. Prior to calling this callback, the engine performs
    /// a host sync, and so the `VkImage` can be used in a pipeline by the embedder
    /// without any additional synchronization.
    /// Not used if a `FlutterCompositor` is supplied in `FlutterProjectArgs`.
    fn present_image(&mut self, image: VulkanImage) -> bool;
}

pub struct VulkanRendererConfig {
    /// The Vulkan API version. This should match the value set in
    /// `VkApplicationInfo::apiVersion` when the `VkInstance` was created.
    pub version: u32,
    /// `VkInstance` handle. Must not be destroyed before `FlutterEngineShutdown` is
    /// called.
    pub instance: sys::FlutterVulkanInstanceHandle,
    /// `VkPhysicalDevice` handle.
    pub physical_device: sys::FlutterVulkanPhysicalDeviceHandle,
    /// `VkDevice` handle. Must not be destroyed before `FlutterEngineShutdown` is
    /// called.
    pub device: sys::FlutterVulkanDeviceHandle,
    /// The queue family index of the `VkQueue` supplied in the next field.
    pub queue_family_index: u32,
    /// `VkQueue` handle.
    /// The queue should not be used without protection from a mutex to make sure
    /// it is not used simultaneously with other threads. That mutex should match
    /// the one injected via the `get_instance_proc_address_callback`.
    /// There is a proposal to remove the need for the mutex at
    /// <https://github.com/flutter/flutter/issues/134573>.
    pub queue: sys::FlutterVulkanQueueHandle,
    /// Array of enabled instance extension names. This should match the names
    /// passed to `VkInstanceCreateInfo.ppEnabledExtensionNames` when the instance
    /// was created, but any subset of enabled instance extensions may be
    /// specified.
    /// This field is optional; `nullptr` may be specified.
    /// This memory is only accessed during the call to `FlutterEngineInitialize`.
    pub enabled_instance_extensions: Vec<CString>,
    /// Array of enabled logical device extension names. This should match the
    /// names passed to `VkDeviceCreateInfo.ppEnabledExtensionNames` when the
    /// logical device was created, but any subset of enabled logical device
    /// extensions may be specified.
    /// This field is optional; `nullptr` may be specified.
    /// This memory is only accessed during the call to `FlutterEngineInitialize`.
    /// For example: `VK_KHR_GET_MEMORY_REQUIREMENTS_2_EXTENSION_NAME`
    pub enabled_device_extensions: Vec<CString>,

    pub handler: Box<dyn VulkanRendererHandler>,
}

impl From<VulkanRendererConfig> for super::RendererConfig {
    fn from(config: VulkanRendererConfig) -> Self {
        Self::Vulkan(config)
    }
}

pub(crate) struct VulkanRendererUserData {
    // Vec<CString>.map(CString::into_raw).collect::<Box<[*mut std::ffi::c_char]>>().into_raw()
    enabled_instance_extensions: *mut [*mut std::ffi::c_char],
    enabled_device_extensions: *mut [*mut std::ffi::c_char],

    handler: Box<dyn VulkanRendererHandler>,
}

impl Drop for VulkanRendererUserData {
    fn drop(&mut self) {
        // just .into_iter() gives me Iterator<Item = &*mut c_char>
        // so we do .into_vec().into_iter() to get Iterator<Item = *mut c_char>

        unsafe { Box::from_raw(self.enabled_instance_extensions) }
            .into_vec()
            .into_iter()
            .map(|raw| unsafe { CString::from_raw(raw) })
            .for_each(drop);

        unsafe { Box::from_raw(self.enabled_device_extensions) }
            .into_vec()
            .into_iter()
            .map(|raw| unsafe { CString::from_raw(raw) })
            .for_each(drop);
    }
}

mod callbacks {

    use crate::{EngineUserData, RendererUserData};

    use super::*;

    pub extern "C" fn get_instance_proc_address(
        engine_user_data: *mut std::ffi::c_void,
        instance: sys::FlutterVulkanInstanceHandle,
        name: *const std::os::raw::c_char,
    ) -> *mut std::ffi::c_void {
        let engine_user_data = engine_user_data.cast::<EngineUserData>();
        let engine_user_data = unsafe { &mut *engine_user_data };

        let RendererUserData::Vulkan(user_data) = &mut engine_user_data.renderer_user_data else {
            unreachable!("Vulkan renderer callback called with non-vulkan renderer user data.");
        };

        user_data
            .handler
            .get_instance_proc_address(instance, unsafe { CStr::from_ptr(name) })
    }

    pub extern "C" fn get_next_image(
        engine_user_data: *mut std::ffi::c_void,
        frame_info: *const sys::FlutterFrameInfo,
    ) -> sys::FlutterVulkanImage {
        let engine_user_data = engine_user_data.cast::<EngineUserData>();
        let engine_user_data = unsafe { &mut *engine_user_data };

        let RendererUserData::Vulkan(user_data) = &mut engine_user_data.renderer_user_data else {
            unreachable!("Vulkan renderer callback called with non-vulkan renderer user data.");
        };

        let frame_info = FrameInfo::from(unsafe { *frame_info });

        user_data.handler.get_next_image(frame_info).into()
    }

    pub extern "C" fn present_image(
        engine_user_data: *mut std::ffi::c_void,
        image: *const sys::FlutterVulkanImage,
    ) -> bool {
        let engine_user_data = engine_user_data.cast::<EngineUserData>();
        let engine_user_data = unsafe { &mut *engine_user_data };

        let RendererUserData::Vulkan(user_data) = &mut engine_user_data.renderer_user_data else {
            unreachable!("Vulkan renderer callback called with non-vulkan renderer user data.");
        };

        let image = VulkanImage::from(unsafe { *image });

        user_data.handler.present_image(image)
    }
    const _: sys::FlutterVulkanInstanceProcAddressCallback = Some(get_instance_proc_address);
    const _: sys::FlutterVulkanImageCallback = Some(get_next_image);
    const _: sys::FlutterVulkanPresentCallback = Some(present_image);
}

impl From<VulkanRendererConfig> for (VulkanRendererUserData, sys::FlutterVulkanRendererConfig) {
    fn from(vulkan: VulkanRendererConfig) -> Self {
        let enabled_instance_extensions: *mut [*mut std::ffi::c_char] = Box::into_raw(
            vulkan
                .enabled_instance_extensions
                .into_iter()
                .map(CString::into_raw)
                .collect::<Box<_>>(),
        );

        let enabled_device_extensions: *mut [*mut std::ffi::c_char] = Box::into_raw(
            vulkan
                .enabled_device_extensions
                .into_iter()
                .map(CString::into_raw)
                .collect::<Box<_>>(),
        );

        (
            VulkanRendererUserData {
                enabled_instance_extensions,
                enabled_device_extensions,
                handler: vulkan.handler,
            },
            sys::FlutterVulkanRendererConfig {
                struct_size: std::mem::size_of::<sys::FlutterVulkanRendererConfig>(),
                version: vulkan.version,
                instance: vulkan.instance,
                physical_device: vulkan.physical_device,
                device: vulkan.device,
                queue_family_index: vulkan.queue_family_index,
                queue: vulkan.queue,

                enabled_instance_extension_count: enabled_instance_extensions.len(),
                enabled_instance_extensions: enabled_instance_extensions
                    .cast::<*const std::ffi::c_char>(),
                enabled_device_extension_count: enabled_device_extensions.len(),
                enabled_device_extensions: enabled_device_extensions
                    .cast::<*const std::ffi::c_char>(),

                get_instance_proc_address_callback: Some(callbacks::get_instance_proc_address),
                get_next_image_callback: Some(callbacks::get_next_image),
                present_image_callback: Some(callbacks::present_image),
            },
        )
    }
}
