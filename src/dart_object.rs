use std::ffi::CStr;

use crate::{sys, Engine};

pub enum DartObject<'a> {
    Null,
    Bool(bool),
    Int32(i32),
    Int64(i64),
    Double(f64),
    String(&'a CStr),
    /// A buffer which is copied into the engine when posted.
    /// Corresponds to a `Uint8List` which is fully owned by Dart.
    Buffer(&'a [u8]),
    /// A buffer which is not copied, and shared with the engine.
    /// Corresponds to a `Uint8List` which is owend by Rust, but shared with Dart.
    ///
    /// It can be modified by Dart, and as such it can only take a raw pointer to the data.
    /// It is the caller's responsibility to prevent data races,
    /// and to ensure that the data is not deallocated before the engine is done with it.
    ///
    /// The `collect` callback is called when the engine no longer needs this buffer.
    /// It can then be safely deallocated.
    SharedBuffer {
        data: *mut [u8],
        collect: Box<dyn FnOnce()>,
    },
}

impl Engine {
    /// Posts a Dart object to specified send port.
    /// The corresponding receive port for send port can be
    /// in any isolate running in the VM.
    /// This isolate can also be the root isolate for an unrelated engine.
    /// The engine parameter is necessary only to ensure the call is
    /// not made when no engine (and hence no VM) is running.
    ///
    /// Unlike the platform messages mechanism, there are no threading
    /// restrictions when using this API. Message can be posted on any
    /// thread and they will be made available to isolate on which the
    /// corresponding send port is listening.
    ///
    /// Returns if the message was posted to the send port.
    pub fn post_dart_object(
        &mut self,
        port: sys::FlutterEngineDartPort,
        object: DartObject,
    ) -> crate::Result<()> {
        let buffer: sys::FlutterEngineDartBuffer;
        let object = match object {
            DartObject::Null => sys::FlutterEngineDartObject {
                type_: sys::FlutterEngineDartObjectType::Null,
                // technically this should be uninitialized, i think?
                // but like, uninit memory is a footgun, so let's just set it to a null pointer
                __bindgen_anon_1: sys::FlutterEngineDartObject__bindgen_ty_1 {
                    string_value: std::ptr::null(),
                },
            },
            DartObject::Bool(bool_value) => sys::FlutterEngineDartObject {
                type_: sys::FlutterEngineDartObjectType::Bool,
                __bindgen_anon_1: sys::FlutterEngineDartObject__bindgen_ty_1 { bool_value },
            },
            DartObject::Int32(int32_value) => sys::FlutterEngineDartObject {
                type_: sys::FlutterEngineDartObjectType::Int32,
                __bindgen_anon_1: sys::FlutterEngineDartObject__bindgen_ty_1 { int32_value },
            },
            DartObject::Int64(int64_value) => sys::FlutterEngineDartObject {
                type_: sys::FlutterEngineDartObjectType::Int64,
                __bindgen_anon_1: sys::FlutterEngineDartObject__bindgen_ty_1 { int64_value },
            },
            DartObject::Double(double_value) => sys::FlutterEngineDartObject {
                type_: sys::FlutterEngineDartObjectType::Double,
                __bindgen_anon_1: sys::FlutterEngineDartObject__bindgen_ty_1 { double_value },
            },
            DartObject::String(string_value) => sys::FlutterEngineDartObject {
                type_: sys::FlutterEngineDartObjectType::String,
                __bindgen_anon_1: sys::FlutterEngineDartObject__bindgen_ty_1 {
                    string_value: string_value.as_ptr(),
                },
            },
            DartObject::Buffer(buf) => {
                buffer = sys::FlutterEngineDartBuffer {
                    struct_size: std::mem::size_of::<sys::FlutterEngineDartBuffer>(),
                    user_data: std::ptr::null_mut(),
                    buffer_collect_callback: None,
                    // SAFETY: when `buffer_collect_callback` is `None`, then the engine pinky promises to treat this as immutable
                    buffer: buf.as_ptr().cast_mut(),
                    buffer_size: buf.len(),
                };
                sys::FlutterEngineDartObject {
                    type_: sys::FlutterEngineDartObjectType::Buffer,
                    __bindgen_anon_1: sys::FlutterEngineDartObject__bindgen_ty_1 {
                        buffer_value: &raw const buffer,
                    },
                }
            }
            DartObject::SharedBuffer { data, collect } => {
                struct UserData {
                    collect: Box<dyn FnOnce()>,
                }

                unsafe extern "C" fn buffer_collect(user_data: *mut std::ffi::c_void) {
                    let user_data = user_data.cast::<UserData>();
                    let user_data = unsafe { Box::from_raw(user_data) };
                    (user_data.collect)()
                }
                const _: sys::VoidCallback = Some(buffer_collect);

                let user_data = Box::new(UserData { collect });
                let user_data = Box::into_raw(user_data);

                buffer = sys::FlutterEngineDartBuffer {
                    struct_size: std::mem::size_of::<sys::FlutterEngineDartBuffer>(),
                    user_data: user_data.cast::<std::ffi::c_void>(),
                    buffer_collect_callback: Some(buffer_collect),
                    buffer: data.cast::<u8>(),
                    buffer_size: data.len(),
                };
                sys::FlutterEngineDartObject {
                    type_: sys::FlutterEngineDartObjectType::Buffer,
                    __bindgen_anon_1: sys::FlutterEngineDartObject__bindgen_ty_1 {
                        buffer_value: &raw const buffer,
                    },
                }
            }
        };

        unsafe { sys::PostDartObject(self.inner.engine, port, &raw const object) }.to_result()
    }
}
