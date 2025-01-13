use std::ffi::{CStr, CString};

use crate::{sys, Engine, Rect, Transformation};

simple_enum! {
    pub enum TextDirection(sys::FlutterTextDirection) {
        /// Text has unknown text direction.
        Unknown,
        /// Text is read from right to left.
        RTL,
        /// Text is read from left to right.
        LTR,
    }
}

bitfield! {
    /// Additional accessibility features that may be enabled by the platform.
    pub struct AccessibilityFeature(sys::FlutterAccessibilityFeature) {
        /// Indicate there is a running accessibility service which is changing the
        /// interaction model of the device.
        AccessibleNavigation,
        /// Indicate the platform is inverting the colors of the application.
        InvertColors,
        /// Request that animations be disabled or simplified.
        DisableAnimations,
        /// Request that text be rendered at a bold font weight.
        BoldText,
        /// Request that certain animations be simplified and parallax effects
        /// removed.
        ReduceMotion,
        /// Request that UI be rendered with darker colors.
        HighContrast,
        /// Request to show on/off labels inside switches.
        OnOffSwitchLabels,
    }

    /// The set of possible actions that can be conveyed to a semantics node.
    pub struct SemanticsAction(sys::FlutterSemanticsAction) {
        /// The equivalent of a user briefly tapping the screen with the finger
        /// without moving it.
        Tap,
        /// The equivalent of a user pressing and holding the screen with the finger
        /// for a few seconds without moving it.
        LongPress,
        /// The equivalent of a user moving their finger across the screen from right
        /// to left.
        ScrollLeft,
        /// The equivalent of a user moving their finger across the screen from left
        /// to
        /// right.
        ScrollRight,
        /// The equivalent of a user moving their finger across the screen from bottom
        /// to top.
        ScrollUp,
        /// The equivalent of a user moving their finger across the screen from top to
        /// bottom.
        ScrollDown,
        /// Increase the value represented by the semantics node.
        Increase,
        /// Decrease the value represented by the semantics node.
        Decrease,
        /// A request to fully show the semantics node on screen.
        ShowOnScreen,
        /// Move the cursor forward by one character.
        MoveCursorForwardByCharacter,
        /// Move the cursor backward by one character.
        MoveCursorBackwardByCharacter,
        /// Set the text selection to the given range.
        SetSelection,
        /// Copy the current selection to the clipboard.
        Copy,
        /// Cut the current selection and place it in the clipboard.
        Cut,
        /// Paste the current content of the clipboard.
        Paste,
        /// Indicate that the node has gained accessibility focus.
        DidGainAccessibilityFocus,
        /// Indicate that the node has lost accessibility focus.
        DidLoseAccessibilityFocus,
        /// Indicate that the user has invoked a custom accessibility action.
        CustomAction,
        /// A request that the node should be dismissed.
        Dismiss,
        /// Move the cursor forward by one word.
        MoveCursorForwardByWord,
        /// Move the cursor backward by one word.
        MoveCursorBackwardByWord,
        /// Replace the current text in the text field.
        SetText,
        /// Request that the respective focusable widget gain input focus.
        Focus,
    }

    /// The set of properties that may be associated with a semantics node.
    pub struct SemanticsFlag(sys::FlutterSemanticsFlag) {
        /// The semantics node has the quality of either being "checked" or
        /// "unchecked".
        HasCheckedState,
        /// Whether a semantics node is checked.
        IsChecked,
        /// Whether a semantics node is selected.
        IsSelected,
        /// Whether the semantic node represents a button.
        IsButton,
        /// Whether the semantic node represents a text field.
        IsTextField,
        /// Whether the semantic node currently holds the user's focus.
        IsFocused,
        /// The semantics node has the quality of either being "enabled" or
        /// "disabled".
        HasEnabledState,
        /// Whether a semantic node that hasEnabledState is currently enabled.
        IsEnabled,
        /// Whether a semantic node is in a mutually exclusive group.
        IsInMutuallyExclusiveGroup,
        /// Whether a semantic node is a header that divides content into sections.
        IsHeader,
        /// Whether the value of the semantics node is obscured.
        IsObscured,
        /// Whether the semantics node is the root of a subtree for which a route name
        /// should be announced.
        ScopesRoute,
        /// Whether the semantics node label is the name of a visually distinct route.
        NamesRoute,
        /// Whether the semantics node is considered hidden.
        IsHidden,
        /// Whether the semantics node represents an image.
        IsImage,
        /// Whether the semantics node is a live region.
        IsLiveRegion,
        /// The semantics node has the quality of either being "on" or "off".
        HasToggledState,
        /// If true, the semantics node is "on". If false, the semantics node is
        /// "off".
        IsToggled,
        /// Whether the platform can scroll the semantics node when the user attempts
        /// to move the accessibility focus to an offscreen child.
        ///
        /// For example, a `ListView` widget has implicit scrolling so that users can
        /// easily move the accessibility focus to the next set of children. A
        /// `PageView` widget does not have implicit scrolling, so that users don't
        /// navigate to the next page when reaching the end of the current one.
        HasImplicitScrolling,
        /// Whether the value of the semantics node is coming from a multi-line text
        /// field.
        ///
        /// This is used for text fields to distinguish single-line text fields from
        /// multi-line ones.
        IsMultiline,
        /// Whether the semantic node is read only.
        ///
        /// Only applicable when [SemanticsFlag::IsTextField] is on.
        IsReadOnly,
        /// Whether the semantic node can hold the user's focus.
        IsFocusable,
        /// Whether the semantics node represents a link.
        IsLink,
        /// Whether the semantics node represents a slider.
        IsSlider,
        /// Whether the semantics node represents a keyboard key.
        IsKeyboardKey,
        /// Whether the semantics node represents a tristate checkbox in mixed state.
        IsCheckStateMixed,
        /// The semantics node has the quality of either being "expanded" or
        /// "collapsed".
        HasExpandedState,
        /// Whether a semantic node that hasExpandedState is currently expanded.
        IsExpanded,
    }
}

// std::range::Range<usize> over std::ops::Range<usize>; but it's currently unstable.
type TextRange<T = usize> = std::ops::Range<T>;

/// Indicates how the assistive technology should treat the string.
///
/// See: <https://api.flutter.dev/flutter/dart-ui/StringAttribute-class.html>
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StringAttribute {
    /// Indicates the assistive technology should announce out the string character
    /// by character.
    ///
    /// See: <https://api.flutter.dev/flutter/dart-ui/SpellOutStringAttribute-class.html>
    SpellOut {
        /// The range of characters for which this attribute applies.
        range: TextRange,
    },
    /// Indicates the assistive technology should announce the string using the
    /// specified locale.
    ///
    /// See: <https://api.flutter.dev/flutter/dart-ui/LocaleStringAttribute-class.html>
    Locale {
        /// The range of characters for which this attribute applies.
        range: TextRange,
        /// The locale of the text.
        locale: CString,
    },
}

impl StringAttribute {
    #[must_use]
    pub fn range(&self) -> TextRange {
        let (StringAttribute::SpellOut { range } | StringAttribute::Locale { range, .. }) = self;

        range.clone()
    }

    pub(crate) fn from_raw(raw: &sys::FlutterStringAttribute) -> Self {
        let range = raw.start..raw.end;

        match raw.type_ {
            sys::FlutterStringAttributeType::SpellOut => {
                let spell_out = unsafe { &*raw.__bindgen_anon_1.spell_out };
                let _ = spell_out; // this struct is actually just empty; but to be clear, it does *exist*.

                Self::SpellOut { range }
            }
            sys::FlutterStringAttributeType::Locale => {
                let locale = unsafe { &*raw.__bindgen_anon_1.locale };
                let locale = unsafe { CStr::from_ptr(locale.locale) }.to_owned();

                Self::Locale { range, locale }
            }
            _ => panic!("Unknown FlutterStringAttributeType: {:?}", raw.type_),
        }
    }
}

/// A string with associated semantic attributes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AttributedString {
    /// The string to be annotated.
    pub string: CString,
    /// The attributes to be applied to the string.
    pub attributes: Vec<StringAttribute>,
}

impl AttributedString {
    fn from_raw(
        string: *const std::ffi::c_char,
        attribute_count: usize,
        attributes: *mut *const sys::FlutterStringAttribute,
    ) -> Self {
        Self {
            string: unsafe { CStr::from_ptr(string) }.to_owned(),
            attributes: unsafe { crate::util::slice_from_raw_parts_with_invalid_empty(attributes, attribute_count) }
                .iter()
                .copied()
                .map(|raw| unsafe { &*raw })
                .map(StringAttribute::from_raw)
                .collect(),
        }
    }
}

/// A node in the Flutter semantics tree.
///
/// The semantics tree is maintained during the semantics phase of the pipeline
/// (i.e., during PipelineOwner.flushSemantics), which happens after
/// compositing. Updates are then pushed to embedders via the registered
/// `FlutterUpdateSemanticsCallback2`.
///
/// See: <https://api.flutter.dev/flutter/semantics/SemanticsNode-class.html>
pub struct SemanticsNode {
    /// The unique identifier for this node.
    pub id: i32,
    /// The set of semantics flags associated with this node.
    pub flags: SemanticsFlag,
    /// The set of semantics actions applicable to this node.
    pub actions: SemanticsAction,

    /// Range of text that is selected.
    pub text_selection: TextRange<i32>,
    /// The total number of scrollable children that contribute to semantics.
    pub scroll_child_count: i32,
    /// The index of the first visible semantic child of a scroll node.
    pub scroll_index: i32,
    /// The current scrolling position in logical pixels if the node is
    /// scrollable.
    pub scroll_position: f64,
    /// The maximum in-range value for `scrollPosition` if the node is scrollable.
    pub scroll_extent_max: f64,
    /// The minimum in-range value for `scrollPosition` if the node is scrollable.
    pub scroll_extent_min: f64,
    /// The elevation along the z-axis at which the rect of this semantics node is
    /// located above its parent.
    pub elevation: f64,
    /// Describes how much space the semantics node takes up along the z-axis.
    pub thickness: f64,
    /// A textual description of the node.
    pub label: AttributedString,
    /// A brief description of the result of performing an action on the node.
    pub hint: AttributedString,
    /// A textual description of the current value of the node.
    pub value: AttributedString,
    /// A value that `value` will have after a [`SemanticsAction::Increase`] action has been performed.
    pub increased_value: AttributedString,
    /// A value that `value` will have after a [`SemanticsAction::Decrease`] action has been performed.
    pub decreased_value: AttributedString,
    /// The reading direction for `label`, `value`, `hint`, `increasedValue`,
    /// `decreasedValue`, and `tooltip`.
    pub text_direction: TextDirection,
    /// The bounding box for this node in its coordinate system.
    pub rect: Rect<f64>,
    /// The transform from this node's coordinate system to its parent's
    /// coordinate system.
    pub transform: Transformation<f64>,
    /// The number of children this node has.
    pub child_count: usize,
    /// Array of child node IDs in traversal order. Guaranteed to have length `child_count`.
    pub children_in_traversal_order: Vec<i32>,
    /// Array of child node IDs in hit test order. Guaranteed to have length `child_count`.
    pub children_in_hit_test_order: Vec<i32>,
    /// Array of `FlutterSemanticsCustomAction` IDs associated with this node.
    pub custom_accessibility_actions: Vec<i32>,
    /// Identifier of the platform view associated with this semantics node, or
    /// -1 if none.
    pub platform_view_id: sys::FlutterPlatformViewIdentifier,
    /// A textual tooltip attached to the node.
    pub tooltip: CString,
}

impl SemanticsNode {
    pub(crate) fn from_raw(raw: &sys::FlutterSemanticsNode2) -> Self {
        Self {
            id: raw.id,
            flags: raw.flags.into(),
            actions: raw.actions.into(),
            text_selection: raw.text_selection_base..raw.text_selection_extent,
            scroll_child_count: raw.scroll_child_count,
            scroll_index: raw.scroll_index,
            scroll_position: raw.scroll_position,
            scroll_extent_max: raw.scroll_extent_max,
            scroll_extent_min: raw.scroll_extent_min,
            elevation: raw.elevation,
            thickness: raw.thickness,
            label: AttributedString::from_raw(
                raw.label,
                raw.label_attribute_count,
                raw.label_attributes,
            ),
            hint: AttributedString::from_raw(
                raw.hint,
                raw.hint_attribute_count,
                raw.hint_attributes,
            ),
            value: AttributedString::from_raw(
                raw.value,
                raw.value_attribute_count,
                raw.value_attributes,
            ),
            increased_value: AttributedString::from_raw(
                raw.increased_value,
                raw.increased_value_attribute_count,
                raw.increased_value_attributes,
            ),
            decreased_value: AttributedString::from_raw(
                raw.decreased_value,
                raw.decreased_value_attribute_count,
                raw.decreased_value_attributes,
            ),
            text_direction: raw
                .text_direction
                .try_into()
                .expect("FlutterTextDirection should always be RTL, LTR, or Unknown"),
            rect: raw.rect.into(),
            transform: raw.transform.into(),
            child_count: raw.child_count,
            children_in_traversal_order: unsafe {
                crate::util::slice_from_raw_parts_with_invalid_empty(raw.children_in_traversal_order, raw.child_count)
            }
            .to_vec(),
            children_in_hit_test_order: unsafe {
                crate::util::slice_from_raw_parts_with_invalid_empty(raw.children_in_hit_test_order, raw.child_count)
            }
            .to_vec(),
            custom_accessibility_actions: unsafe {
                crate::util::slice_from_raw_parts_with_invalid_empty(
                    raw.custom_accessibility_actions,
                    raw.custom_accessibility_actions_count,
                )
            }
            .to_vec(),
            platform_view_id: raw.platform_view_id,
            tooltip: unsafe { CStr::from_ptr(raw.tooltip) }.to_owned(),
        }
    }
}

/// A custom semantics action, or action override.
///
/// Custom actions can be registered by applications in order to provide
/// semantic actions other than the standard actions available through the
/// `FlutterSemanticsAction` enum.
///
/// Action overrides are custom actions that the application developer requests
/// to be used in place of the standard actions in the `FlutterSemanticsAction`
/// enum.
///
/// See: <https://api.flutter.dev/flutter/semantics/CustomSemanticsAction-class.html>
pub struct SemanticsCustomAction {
    /// The unique custom action or action override ID.
    pub id: i32,
    /// For overridden standard actions, corresponds to the
    /// `FlutterSemanticsAction` to override.
    pub override_action: SemanticsAction,
    /// The user-readable name of this custom semantics action.
    pub label: CString,
    /// The hint description of this custom semantics action.
    pub hint: CString,
}

impl SemanticsCustomAction {
    pub(crate) fn from_raw(raw: &sys::FlutterSemanticsCustomAction2) -> Self {
        Self {
            id: raw.id,
            override_action: raw.override_action.into(),
            label: unsafe { CStr::from_ptr(raw.label) }.to_owned(),
            hint: unsafe { CStr::from_ptr(raw.hint) }.to_owned(),
        }
    }
}

pub struct SemanticsUpdate {
    pub nodes: Vec<SemanticsNode>,
    pub custom_actions: Vec<SemanticsCustomAction>,
}

impl SemanticsUpdate {
    pub(crate) fn from_raw(raw: &sys::FlutterSemanticsUpdate2) -> Self {
        Self {
            nodes: unsafe { crate::util::slice_from_raw_parts_with_invalid_empty(raw.nodes, raw.node_count) }
                .iter()
                .copied()
                .map(|raw| unsafe { &*raw })
                .map(SemanticsNode::from_raw)
                .collect(),
            custom_actions: unsafe {
                crate::util::slice_from_raw_parts_with_invalid_empty(raw.custom_actions, raw.custom_action_count)
            }
            .iter()
            .copied()
            .map(|raw| unsafe { &*raw })
            .map(SemanticsCustomAction::from_raw)
            .collect(),
        }
    }
}

impl Engine {
    /// Enable or disable accessibility semantics.
    ///
    /// When enabled, changes to the semantic contents of the window are sent via the
    /// [`EngineHandler::update_semantics`] callback passed in [`FlutterProjectArgs`].
    pub fn update_semantics_enabled(&mut self, enabled: bool) -> crate::Result<()> {
        unsafe { sys::UpdateSemanticsEnabled(self.inner.engine, enabled) }.to_result()
    }

    /// Sets additional accessibility features.
    pub fn update_accessibility_features(
        &mut self,
        features: AccessibilityFeature,
    ) -> crate::Result<()> {
        unsafe { sys::UpdateAccessibilityFeatures(self.inner.engine, features.into()) }.to_result()
    }

    /// Dispatch a semantics action to the specified semantics node.
    pub fn dispatch_semantics_action(
        &mut self,
        node_id: u64,
        action: SemanticsAction,
        data: &[u8],
    ) -> crate::Result<()> {
        unsafe {
            sys::DispatchSemanticsAction(
                self.inner.engine,
                node_id,
                action.into(),
                data.as_ptr(),
                data.len(),
            )
        }
        .to_result()
    }
}
