use crate::{
    sys, BackingStore, BackingStoreConfig, PlatformViewMutation, Point, Region, Size, ViewId,
};

pub trait CompositorHandler: Send + Sync {
    /// A callback invoked by the engine to obtain a backing store for a specific
    /// `FlutterLayer`.
    fn create_backing_store(&mut self, config: BackingStoreConfig) -> Option<BackingStore>;

    /// A callback invoked by the engine to release the backing store. The
    /// embedder may collect any resources associated with the backing store.
    ///
    /// The callback should return true if the operation was successful.
    fn collect_backing_store(&mut self, backing_store: BackingStore) -> bool;

    /// Callback invoked by the engine to composite the contents of each layer
    /// onto the specified view.
    ///
    /// The callback should return true if the operation was successful.
    fn present_view(&mut self, view_id: ViewId, layers: &[Layer]) -> bool;
}

pub struct Compositor {
    /// Avoid caching backing stores provided by this compositor.
    pub avoid_backing_store_cache: bool,

    pub handler: Box<dyn CompositorHandler>,
}

pub(crate) struct CompositorUserData {
    handler: Box<dyn CompositorHandler>,
}

pub struct Layer {
    /// The offset of this layer (in physical pixels) relative to the top left of
    /// the root surface used by the engine.
    pub offset: Point<f64>,
    /// The size of the layer (in physical pixels).
    pub size: Size<f64>,

    /// Indicates whether the contents of a layer are rendered by Flutter or the embedder.
    pub content: LayerContent,

    /// Time in nanoseconds at which this frame is scheduled to be presented. 0 if not known.
    /// See [`crate::Engine::get_current_time`].
    pub presentation_time: u64,
}
pub enum LayerContent {
    /// Indicates that the contents of this layer are rendered by Flutter into a backing store.
    BackingStore(BackingStore, BackingStorePresentInfo),
    /// Indicates that the contents of this layer are determined by the embedder.
    PlatformView(PlatformView),
}
impl Layer {
    #[must_use]
    pub(crate) fn from_raw(raw: &sys::FlutterLayer) -> Self {
        Self {
            offset: raw.offset.into(),
            size: raw.size.into(),
            content: match raw.type_ {
                sys::FlutterLayerContentType::BackingStore => {
                    // SAFETY: checked the discriminant above
                    let backing_store = unsafe { &*raw.__bindgen_anon_1.backing_store };
                    // SAFETY: this one is somewhat subtle, but it's nonnull whenever tpye tpye is backing store
                    // and it's null when not. So, this is safe.
                    let backing_store_present_info = unsafe { &*raw.backing_store_present_info };
                    LayerContent::BackingStore(
                        BackingStore::from_raw(backing_store),
                        BackingStorePresentInfo::from_raw(backing_store_present_info),
                    )
                }
                sys::FlutterLayerContentType::PlatformView => {
                    // SAFETY: checked the discriminant above
                    let platform_view = unsafe { &*raw.__bindgen_anon_1.platform_view };
                    LayerContent::PlatformView(PlatformView::from_raw(platform_view))
                }
                _ => unreachable!("Unknown FlutterLayerContentType; cannot construct a Layer. That enum shouldn't ever be extended; this is probably a bug in the Flutter engine."),
            },
            presentation_time: raw.presentation_time,
        }
    }
}

pub struct BackingStorePresentInfo {
    // The area of the backing store that contains Flutter contents.
    // Pixels outside of this area are transparent and the embedder may choose not to render them.
    // Coordinates are in physical pixels.
    pub paint_region: Region,
}

impl BackingStorePresentInfo {
    pub(crate) fn from_raw(raw: &sys::FlutterBackingStorePresentInfo) -> Self {
        Self {
            paint_region: Region::from_raw(unsafe { &*raw.paint_region }),
        }
    }
}

pub struct PlatformView {
    /// The identifier of this platform view. This identifier is specified by the
    /// application when a platform view is added to the scene via the
    /// `SceneBuilder.addPlatformView` call.
    pub identifier: sys::FlutterPlatformViewIdentifier,
    /// The mutations to be applied by this platform view before it is composited
    /// on-screen. The Flutter application may transform the platform view but
    /// these transformations cannot be affected by the Flutter compositor because
    /// it does not render platform views. Since the embedder is responsible for
    /// composition of these views, it is also the embedder's responsibility to
    /// affect the appropriate transformation.
    ///
    /// The mutations must be applied in order. The mutations done in the
    /// collection don't take into account the device pixel ratio or the root
    /// surface transformation. If these exist, the first mutation in the list
    /// will be a transformation mutation to make sure subsequent mutations are in
    /// the correct coordinate space.
    pub mutations: Vec<PlatformViewMutation>,
}

impl PlatformView {
    pub(crate) fn from_raw(raw: &sys::FlutterPlatformView) -> Self {
        Self {
            identifier: raw.identifier,
            mutations: unsafe {
                crate::util::slice_from_raw_parts_with_invalid_empty(
                    raw.mutations,
                    raw.mutations_count,
                )
            }
            .iter()
            .copied()
            .map(|raw| unsafe { *raw })
            .map(PlatformViewMutation::from)
            .collect(),
        }
    }
}

mod callbacks {

    use super::*;

    pub extern "C" fn create_backing_store(
        backing_store_config: *const sys::FlutterBackingStoreConfig,
        backing_store_out: *mut sys::FlutterBackingStore,
        user_data: *mut std::ffi::c_void,
    ) -> bool {
        let user_data = user_data.cast::<CompositorUserData>();

        let user_data = unsafe { &mut *user_data };

        let backing_store_config = BackingStoreConfig::from(unsafe { *backing_store_config });

        let backing_store = user_data.handler.create_backing_store(backing_store_config);

        unsafe { crate::util::return_out_param(backing_store_out, backing_store) }
    }

    pub extern "C" fn collect_backing_store(
        backing_store: *const sys::FlutterBackingStore,
        user_data: *mut std::ffi::c_void,
    ) -> bool {
        let user_data = user_data.cast::<CompositorUserData>();
        let user_data = unsafe { &mut *user_data };

        let backing_store = BackingStore::from_raw(unsafe { &*backing_store });

        user_data.handler.collect_backing_store(backing_store)
    }

    pub extern "C" fn present_view(present_view_info: *const sys::FlutterPresentViewInfo) -> bool {
        let present_view_info = unsafe { &*present_view_info };

        let user_data = present_view_info.user_data.cast::<CompositorUserData>();
        let user_data = unsafe { &mut *user_data };

        let layers: Box<[Layer]> = unsafe {
            crate::util::slice_from_raw_parts_with_invalid_empty(
                present_view_info.layers,
                present_view_info.layers_count,
            )
        }
        .iter()
        .copied()
        .map(|raw| unsafe { &*raw })
        .map(Layer::from_raw)
        .collect();

        user_data
            .handler
            .present_view(ViewId(present_view_info.view_id), &layers)
    }
    const _: sys::FlutterBackingStoreCreateCallback = Some(create_backing_store);
    const _: sys::FlutterBackingStoreCollectCallback = Some(collect_backing_store);
    const _: sys::FlutterPresentViewCallback = Some(present_view);
}

impl From<Compositor> for (*mut CompositorUserData, sys::FlutterCompositor) {
    fn from(compositor: Compositor) -> Self {
        let user_data = Box::new(CompositorUserData {
            handler: compositor.handler,
        });
        let user_data = Box::into_raw(user_data);

        (
            user_data,
            sys::FlutterCompositor {
                struct_size: std::mem::size_of::<sys::FlutterCompositor>(),
                user_data: user_data.cast::<std::ffi::c_void>(),
                create_backing_store_callback: Some(callbacks::create_backing_store),
                collect_backing_store_callback: Some(callbacks::collect_backing_store),
                avoid_backing_store_cache: compositor.avoid_backing_store_cache,
                present_layers_callback: None, // deprecated; superceded by present_view_callback
                present_view_callback: Some(callbacks::present_view),
            },
        )
    }
}
