use crate::sys;

simple_enum! {
    /// A pixel format to be used for software rendering.
    ///
    /// A single pixel always stored as a POT number of bytes. (so in practice
    /// either 1, 2, 4, 8, 16 bytes per pixel)
    ///
    /// There are two kinds of pixel formats:
    ///   - formats where all components are 8 bits, called array formats
    ///     The component order as specified in the pixel format name is the
    ///     order of the components' bytes in memory, with the leftmost component
    ///     occupying the lowest memory address.
    ///
    ///   - all other formats are called packed formats, and the component order
    ///     as specified in the format name refers to the order in the native type.
    ///     for example, for kFlutterSoftwarePixelFormatRGB565, the R component
    ///     uses the 5 least significant bits of the uint16_t pixel value.
    ///
    /// Each pixel format in this list is documented with an example on how to get
    /// the color components from the pixel.
    /// - for packed formats, p is the pixel value as a word. For example, you can
    ///   get the pixel value for a RGB565 formatted buffer like this:
    ///   uint16_t p = ((const uint16_t*) allocation)[row_bytes * y / bpp + x];
    ///   (with bpp being the bytes per pixel, so 2 for RGB565)
    ///
    /// - for array formats, p is a pointer to the pixel value. For example, you
    ///   can get the p for a RGBA8888 formatted buffer like this:
    ///   const uint8_t *p = ((const uint8_t*) allocation) + row_bytes*y + x*4;
    pub enum SoftwarePixelFormat(sys::FlutterSoftwarePixelFormat) {
        /// pixel with 8 bit grayscale value.
        /// The grayscale value is the luma value calculated from r, g, b
        /// according to BT.709. (gray = r*0.2126 + g*0.7152 + b*0.0722)
        Gray8,

        /// pixel with 5 bits red, 6 bits green, 5 bits blue, in 16-bit word.
        ///   r = p & 0x3F; g = (p>>5) & 0x3F; b = p>>11;
        RGB565,

        /// pixel with 4 bits for alpha, red, green, blue; in 16-bit word.
        ///   r = p & 0xF;  g = (p>>4) & 0xF;  b = (p>>8) & 0xF;   a = p>>12;
        RGBA4444,

        /// pixel with 8 bits for red, green, blue, alpha.
        ///   r = p[0]; g = p[1]; b = p[2]; a = p[3];
        RGBA8888,

        /// pixel with 8 bits for red, green and blue and 8 unused bits.
        ///   r = p[0]; g = p[1]; b = p[2];
        RGBX8888,

        /// pixel with 8 bits for blue, green, red and alpha.
        ///   r = p[2]; g = p[1]; b = p[0]; a = p[3];
        BGRA8888,

        /// either [FlutterSoftwarePixelFormat::BGRA8888] or [FlutterSoftwarePixelFormat::RGBA8888]
        /// depending on CPU endianess and OS
        Native32,
    }
}

pub struct SoftwareBackingStore {
    /// A pointer to the raw bytes of the allocation described by this software backing store.
    pub allocation: *const u8,
    /// The number of bytes in a single row of the allocation.
    pub row_bytes: usize,
    /// The number of rows in the allocation.
    pub height: usize,
    /// The pixel format that the engine should use to render into the allocation.
    pub pixel_format: SoftwarePixelFormat,
}

extern "C" fn destroy_software_callback(user_data: *mut std::ffi::c_void) {
    let _ = user_data;
    println!("destroy_software_callback");
}
const _: sys::VoidCallback = Some(destroy_software_callback);

impl From<SoftwareBackingStore> for sys::FlutterSoftwareBackingStore2 {
    fn from(software: SoftwareBackingStore) -> Self {
        Self {
            struct_size: std::mem::size_of::<Self>(),
            user_data: std::ptr::null_mut(),
            destruction_callback: Some(destroy_software_callback),

            allocation: software.allocation as *const std::ffi::c_void,
            row_bytes: software.row_bytes,
            height: software.height,
            pixel_format: software.pixel_format.into(),
        }
    }
}
impl SoftwareBackingStore {
    pub fn from_raw(raw: &sys::FlutterSoftwareBackingStore2) -> Self {
        assert!(raw.destruction_callback == Some(destroy_software_callback),
            "from_raw(&sys::FlutterSoftwareBackingStore2) for a software buffer for which we didn't set the destruction callback"
        );
        Self {
            allocation: raw.allocation as *const u8,
            row_bytes: raw.row_bytes,
            height: raw.height,
            pixel_format: raw.pixel_format.try_into().unwrap(),
        }
    }
}

pub trait SoftwareRendererHandler {
    /// The callback presented to the embedder to present a fully populated buffer to the user.
    /// The pixel format of the buffer is the native 32-bit RGBA format.
    /// The buffer is owned by the Flutter engine and must be copied in this callback if needed.
    fn surface_present(&mut self, allocation: *const u8, row_bytes: usize, height: usize) -> bool;
}

pub struct SoftwareRendererConfig {
    pub handler: Box<dyn SoftwareRendererHandler>,
}

impl From<SoftwareRendererConfig> for super::RendererConfig {
    fn from(config: SoftwareRendererConfig) -> Self {
        Self::Software(config)
    }
}

pub(crate) struct SoftwareRendererUserData {
    handler: Box<dyn SoftwareRendererHandler>,
}

mod callbacks {
    use crate::{EngineUserData, RendererUserData};

    use super::*;

    pub extern "C" fn surface_present(
        engine_user_data: *mut std::ffi::c_void,
        allocation: *const std::ffi::c_void,
        row_bytes: usize,
        height: usize,
    ) -> bool {
        let engine_user_data = engine_user_data as *mut EngineUserData;
        let engine_user_data = unsafe { &mut *engine_user_data };

        let RendererUserData::Software(user_data) = &mut engine_user_data.renderer_user_data else {
            unreachable!("Software renderer callback called with non-software renderer user data.");
        };

        user_data
            .handler
            .surface_present(allocation as *const u8, row_bytes, height)
    }

    const _: sys::SoftwareSurfacePresentCallback = Some(surface_present);
}

impl From<SoftwareRendererConfig>
    for (SoftwareRendererUserData, sys::FlutterSoftwareRendererConfig)
{
    fn from(software: SoftwareRendererConfig) -> Self {
        (
            SoftwareRendererUserData {
                handler: software.handler,
            },
            sys::FlutterSoftwareRendererConfig {
                struct_size: std::mem::size_of::<sys::FlutterSoftwareRendererConfig>(),
                surface_present_callback: Some(callbacks::surface_present),
            },
        )
    }
}
