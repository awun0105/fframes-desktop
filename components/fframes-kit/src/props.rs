//! Editable props: components that fframes Studio can change without editing code.
//!
//! A component becomes editable with `#[derive(Props)]` (feature `studio`) and a key:
//! `ui.show(LowerThird { .. }.editable("speaker"))`, or `<LowerThird editable="speaker" .. />`
//! in [`ui!`](crate::ui!). The values in a [`PropsStore`] then replace the code's values when
//! the component renders, and the component is drawn inside a `<g>` whose id names it for
//! Studio's canvas selection. Without a store, or without the `studio` feature, `editable`
//! renders the component unchanged.
//!
//! The store holds the project's `props.json`. Renders read it with [`PropsStore::load`];
//! Studio's preview worker changes it through the closures a project wires in:
//!
//! ```ignore
//! let props = video.props.clone(); // Arc<PropsStore>
//! props.listen();
//! let (apply, info) = (props.clone(), props.clone());
//! let config = config.with_props(PropsHooks::new(
//!     move |doc| apply.set(doc),
//!     move |identity| info.info(identity),
//! ));
//! ```

use std::borrow::Cow;

#[cfg(not(feature = "studio"))]
use fframes::Svgr;

use crate::ui::Component;
#[cfg(not(feature = "studio"))]
use crate::ui::Ui;

#[cfg(feature = "studio")]
pub use studio::*;

/// A component with an editor key, see the [module docs](self).
pub struct Editable<'a, C> {
    pub key: Cow<'a, str>,
    pub component: C,
}

impl<'a, C> Editable<'a, C> {
    pub fn new(key: impl Into<Cow<'a, str>>, component: C) -> Self {
        Self {
            key: key.into(),
            component,
        }
    }
}

/// `component.editable("key")`, the short form of [`Editable::new`].
pub trait EditableExt<'a>: Sized {
    fn editable(self, key: impl Into<Cow<'a, str>>) -> Editable<'a, Self> {
        Editable::new(key, self)
    }
}

impl<'a, C: Component<'a>> EditableExt<'a> for C {}

#[cfg(not(feature = "studio"))]
impl<'a, C: Component<'a>> Component<'a> for Editable<'a, C> {
    fn render(self, ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
        ui.show(self.component)
    }
}

#[cfg(feature = "studio")]
mod studio {
    use std::borrow::Cow;
    use std::collections::{BTreeMap, HashMap};
    use std::path::Path;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::{Arc, Mutex, PoisonError, RwLock};

    pub use fframes_kit_macros::{PropChoice, Props};
    pub use fframes_studio_protocol::{
        EditorObjectIdentity, PropField, PropKind, PropsDocument, PropsInfo, PropsSchema,
    };
    use serde_json::Value;

    use fframes::diagnostics::{Diagnostic, report};
    use fframes::{Color, EditorObjectKey, Svgr};

    use super::Editable;
    use crate::anim::Anim;
    use crate::ui::{Component, Ui};

    /// Field values as JSON, keyed by field name.
    pub type PropValues = BTreeMap<String, Value>;

    /// The repeat part of every kit identity: keys are unique per scene.
    pub const REPEAT_KEY: &str = "main";

    /// A props struct whose fields Studio can edit. Derive it with `#[derive(Props)]`:
    /// fields whose type implements [`PropValue`] are editable, the others are left out.
    pub trait Props {
        /// The component part of the editor identity, the type's name.
        const COMPONENT: &'static str;
        fn schema() -> PropsSchema;
        /// The editable fields' current values. A field without a value now (an
        /// [`Anim`] holding keyframes) is left out.
        fn values(&self) -> PropValues;
        /// Sets the fields in `values`; returns the names it could not set (unknown fields,
        /// values of the wrong type or out of range).
        fn apply(&mut self, values: &PropValues) -> Vec<String>;
    }

    /// A field type Studio can edit.
    pub trait PropValue {
        fn kind() -> PropKind;
        fn to_value(&self) -> Option<Value>;
        /// Replaces `self` with `value`; `false` when `value` does not fit.
        fn set_value(&mut self, value: &Value) -> bool;
    }

    macro_rules! float_value {
        ($($t:ty),*) => {$(
            impl PropValue for $t {
                fn kind() -> PropKind {
                    PropKind::Number
                }
                fn to_value(&self) -> Option<Value> {
                    serde_json::Number::from_f64(f64::from(*self)).map(Value::Number)
                }
                fn set_value(&mut self, value: &Value) -> bool {
                    match value.as_f64() {
                        Some(v) if v.is_finite() => {
                            *self = v as $t;
                            true
                        }
                        _ => false,
                    }
                }
            }
        )*};
    }
    float_value!(f32, f64);

    macro_rules! integer_value {
        ($($t:ty),*) => {$(
            impl PropValue for $t {
                fn kind() -> PropKind {
                    PropKind::Integer
                }
                fn to_value(&self) -> Option<Value> {
                    Some(Value::from(*self))
                }
                fn set_value(&mut self, value: &Value) -> bool {
                    let parsed = match value {
                        Value::Number(n) => n
                            .as_i64()
                            .and_then(|v| <$t>::try_from(v).ok())
                            .or_else(|| n.as_u64().and_then(|v| <$t>::try_from(v).ok())),
                        _ => None,
                    };
                    parsed.map(|v| *self = v).is_some()
                }
            }
        )*};
    }
    integer_value!(u8, u16, u32, u64, usize, i8, i16, i32, i64, isize);

    impl PropValue for bool {
        fn kind() -> PropKind {
            PropKind::Toggle
        }
        fn to_value(&self) -> Option<Value> {
            Some(Value::Bool(*self))
        }
        fn set_value(&mut self, value: &Value) -> bool {
            value.as_bool().map(|v| *self = v).is_some()
        }
    }

    impl PropValue for String {
        fn kind() -> PropKind {
            PropKind::Text
        }
        fn to_value(&self) -> Option<Value> {
            Some(Value::String(self.clone()))
        }
        fn set_value(&mut self, value: &Value) -> bool {
            value.as_str().map(|v| v.clone_into(self)).is_some()
        }
    }

    impl PropValue for Cow<'_, str> {
        fn kind() -> PropKind {
            PropKind::Text
        }
        fn to_value(&self) -> Option<Value> {
            Some(Value::String(self.to_string()))
        }
        fn set_value(&mut self, value: &Value) -> bool {
            value
                .as_str()
                .map(|v| *self = Cow::Owned(v.to_owned()))
                .is_some()
        }
    }

    /// `#rrggbb`, or `#rrggbbaa` when not opaque.
    pub fn color_to_hex(color: Color) -> String {
        let Color { r, g, b, a } = color;
        if a == 255 {
            format!("#{r:02x}{g:02x}{b:02x}")
        } else {
            format!("#{r:02x}{g:02x}{b:02x}{a:02x}")
        }
    }

    /// Parses `#rrggbb` or `#rrggbbaa`; anything else is `None`.
    pub fn color_from_hex(text: &str) -> Option<Color> {
        let hex = text.strip_prefix('#')?;
        if !matches!(hex.len(), 6 | 8) || !hex.chars().all(|c| c.is_ascii_hexdigit()) {
            return None;
        }
        let byte = |i: usize| u8::from_str_radix(&hex[i..i + 2], 16).ok();
        let alpha = if hex.len() == 8 { byte(6)? } else { 255 };
        Some(Color::rgba(byte(0)?, byte(2)?, byte(4)?, alpha))
    }

    impl PropValue for Color {
        fn kind() -> PropKind {
            PropKind::Color
        }
        fn to_value(&self) -> Option<Value> {
            Some(Value::String(color_to_hex(*self)))
        }
        fn set_value(&mut self, value: &Value) -> bool {
            value
                .as_str()
                .and_then(color_from_hex)
                .map(|v| *self = v)
                .is_some()
        }
    }

    /// `null` is `None`, such as a shape without fill.
    impl<T: PropValue + Default> PropValue for Option<T> {
        fn kind() -> PropKind {
            T::kind()
        }
        fn to_value(&self) -> Option<Value> {
            match self {
                Some(v) => v.to_value(),
                None => Some(Value::Null),
            }
        }
        fn set_value(&mut self, value: &Value) -> bool {
            if value.is_null() {
                *self = None;
                return true;
            }
            let mut v = T::default();
            v.set_value(value).then(|| *self = Some(v)).is_some()
        }
    }

    /// Editable while it holds a fixed value; a set value replaces keyframes.
    impl PropValue for Anim<'_, f32> {
        fn kind() -> PropKind {
            PropKind::Number
        }
        fn to_value(&self) -> Option<Value> {
            match self {
                Anim::Fixed(v) => v.to_value(),
                Anim::Keyframes(_) => None,
            }
        }
        fn set_value(&mut self, value: &Value) -> bool {
            let mut v = 0.0_f32;
            v.set_value(value).then(|| *self = Anim::Fixed(v)).is_some()
        }
    }

    /// What `#[derive(Props)]` expands to. Fields whose type implements [`PropValue`] go
    /// through [`Editable`](self::__private::Editable), the others through
    /// [`NotEditable`](self::__private::NotEditable): method lookup tries `&Slot<T>` before
    /// `&&Slot<T>`, so the first impl wins when it applies.
    #[doc(hidden)]
    pub mod __private {
        use std::cell::RefCell;
        use std::marker::PhantomData;

        pub use serde_json::Value;

        use super::{PropKind, PropValue};

        /// Whether a number lies within a field's `min` and `max`; other values pass, the
        /// field's type checks them.
        pub fn in_range(value: &Value, min: Option<f64>, max: Option<f64>) -> bool {
            value
                .as_f64()
                .is_none_or(|v| min.is_none_or(|min| v >= min) && max.is_none_or(|max| v <= max))
        }

        pub struct Slot<'s, T>(pub RefCell<Option<&'s mut T>>, pub Option<&'s T>);
        pub struct Kind<T>(pub PhantomData<T>);

        impl<'s, T> Slot<'s, T> {
            pub fn get(value: &'s T) -> Self {
                Self(RefCell::new(None), Some(value))
            }
            pub fn set(value: &'s mut T) -> Self {
                Self(RefCell::new(Some(value)), None)
            }
        }

        pub trait Editable {
            fn __get(&self) -> Option<Value>;
            fn __set(&self, value: &Value) -> bool;
        }
        impl<T: PropValue> Editable for Slot<'_, T> {
            fn __get(&self) -> Option<Value> {
                self.1.and_then(PropValue::to_value)
            }
            fn __set(&self, value: &Value) -> bool {
                self.0
                    .borrow_mut()
                    .as_mut()
                    .is_some_and(|slot| slot.set_value(value))
            }
        }

        pub trait NotEditable {
            fn __get(&self) -> Option<Value> {
                None
            }
            fn __set(&self, _: &Value) -> bool {
                false
            }
        }
        impl<T> NotEditable for &Slot<'_, T> {}

        pub trait EditableKind {
            fn __kind(&self) -> Option<PropKind>;
        }
        impl<T: PropValue> EditableKind for Kind<T> {
            fn __kind(&self) -> Option<PropKind> {
                Some(T::kind())
            }
        }
        pub trait NotEditableKind {
            fn __kind(&self) -> Option<PropKind> {
                None
            }
        }
        impl<T> NotEditableKind for &Kind<T> {}

        /// Writes the value of a choice: the variant's name.
        pub fn choice_value(name: &str) -> Value {
            Value::String(name.to_owned())
        }
    }

    /// The props document of a video plus, while Studio listens, how each editable object
    /// last rendered. Shared between render threads; keep it in an `Arc` in the video.
    #[derive(Debug, Default)]
    pub struct PropsStore {
        doc: RwLock<Arc<PropsDocument>>,
        rendered: Mutex<HashMap<EditorObjectIdentity, PropsInfo>>,
        listening: AtomicBool,
    }

    impl PropsStore {
        pub fn new(doc: PropsDocument) -> Self {
            Self {
                doc: RwLock::new(Arc::new(doc)),
                ..Default::default()
            }
        }

        /// The document in `path`; a missing file is an empty document.
        pub fn load(path: impl AsRef<Path>) -> Result<Self, String> {
            let path = path.as_ref();
            match std::fs::read_to_string(path) {
                Ok(text) => PropsDocument::parse(&text)
                    .map(Self::new)
                    .map_err(|error| format!("{}: {error}", path.display())),
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(Self::default()),
                Err(error) => Err(format!("{}: {error}", path.display())),
            }
        }

        /// Replaces the document from the next frame on; `None` restores the code's values.
        pub fn set(&self, doc: Option<PropsDocument>) {
            *self.doc.write().unwrap_or_else(PoisonError::into_inner) =
                Arc::new(doc.unwrap_or_default());
        }

        pub fn document(&self) -> Arc<PropsDocument> {
            self.doc
                .read()
                .unwrap_or_else(PoisonError::into_inner)
                .clone()
        }

        /// Records how editable objects render from now on, for [`info`](Self::info).
        pub fn listen(&self) {
            self.listening.store(true, Ordering::Relaxed);
        }

        /// The schema and values of an object as it last rendered; `None` before
        /// [`listen`](Self::listen) or when it has not rendered since.
        pub fn info(&self, identity: &EditorObjectIdentity) -> Option<PropsInfo> {
            self.rendered
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .get(identity)
                .cloned()
        }

        fn record(&self, identity: EditorObjectIdentity, info: PropsInfo) {
            self.rendered
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .insert(identity, info);
        }
    }

    /// The props a [`Ui`] renders with: one snapshot of the document per `Ui`, shared by
    /// the `Ui`s of its containers.
    #[derive(Clone)]
    pub(crate) struct PropsView<'a> {
        pub store: &'a PropsStore,
        pub doc: Arc<PropsDocument>,
    }

    impl<'a> PropsView<'a> {
        pub fn new(store: &'a PropsStore) -> Self {
            Self {
                store,
                doc: store.document(),
            }
        }
    }

    impl<'a, C: Component<'a> + Props> Component<'a> for Editable<'a, C> {
        fn render(mut self, ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
            let Some(view) = ui.props.clone() else {
                return ui.show(self.component);
            };
            let identity = EditorObjectIdentity {
                scene_instance_key: ui.scene_key().to_owned(),
                component_key: C::COMPONENT.to_owned(),
                object_key: self.key.to_string(),
                repeat_key: REPEAT_KEY.to_owned(),
            };
            let listening = view.store.listening.load(Ordering::Relaxed);
            let code_values = listening.then(|| self.component.values());
            if let Some(values) = view.doc.values(&identity) {
                let rejected = self.component.apply(values);
                if !rejected.is_empty() {
                    report(Diagnostic::RendererWarning {
                        message: format!(
                            "props of {} \"{}\" not applied: {}",
                            C::COMPONENT,
                            self.key,
                            rejected.join(", ")
                        ),
                    });
                }
            }
            if let Some(code_values) = code_values {
                let info = PropsInfo {
                    schema: C::schema(),
                    code_values,
                    values: self.component.values(),
                };
                view.store.record(identity.clone(), info);
            }
            let id = EditorObjectKey::new(
                identity.scene_instance_key,
                identity.component_key,
                identity.object_key,
                identity.repeat_key,
            )
            .ok()
            .map(|key| key.render_id());
            let drawn = ui.show(self.component);
            match id {
                Some(id) => fframes::svgr!(<g id={id}>{drawn}</g>),
                None => drawn,
            }
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn colors_round_trip_through_hex() {
            let color = Color::rgba(0x12, 0xab, 0xff, 0x80);
            assert_eq!(color_to_hex(color), "#12abff80");
            assert_eq!(color_from_hex("#12abff80"), Some(color));
            assert_eq!(color_to_hex(Color::rgb(1, 2, 3)), "#010203");
            for bad in ["12abff", "#12ab", "#12abfg", "#12abff8", ""] {
                assert_eq!(color_from_hex(bad), None, "{bad}");
            }
        }

        #[test]
        fn values_check_their_type_and_range() {
            let mut n = 1.0_f32;
            assert!(n.set_value(&Value::from(2.5)));
            assert_eq!(n, 2.5);
            assert!(!n.set_value(&Value::from("2")));
            let mut count = 3_u8;
            assert!(!count.set_value(&Value::from(300)));
            assert!(!count.set_value(&Value::from(-1)));
            assert!(!count.set_value(&Value::from(1.5)));
            assert!(count.set_value(&Value::from(7)));
            assert_eq!(count, 7);
            let mut text: Cow<str> = Cow::Borrowed("a");
            assert!(text.set_value(&Value::from("b")));
            assert_eq!(text, "b");
            let mut on = false;
            assert!(on.set_value(&Value::from(true)) && on);
            let mut anim: Anim<f32> = Anim::Fixed(1.0);
            assert_eq!(anim.to_value(), Some(Value::from(1.0)));
            assert!(anim.set_value(&Value::from(4.0)));
            assert!(matches!(anim, Anim::Fixed(v) if v == 4.0));
        }

        #[derive(Clone, Copy, Debug, PartialEq, PropChoice)]
        enum Align {
            Left,
            Right,
        }

        #[derive(Props)]
        struct Card<'a> {
            title: Cow<'a, str>,
            #[props(min = 0.0, max = 1920.0, unit = "px")]
            x: f32,
            #[props(skip)]
            hidden: f32,
            count: usize,
            fill: Color,
            visible: bool,
            align: Align,
            opacity: Anim<'a, f32>,
            children: crate::ui::Children<'a>,
            points: &'a [f32],
        }

        fn card<'a>() -> Card<'a> {
            Card {
                title: Cow::Borrowed("Ada"),
                x: 10.0,
                hidden: 1.0,
                count: 2,
                fill: Color::rgb(255, 0, 0),
                visible: true,
                align: Align::Left,
                opacity: Anim::Fixed(0.5),
                children: crate::ui::no_children(),
                points: &[],
            }
        }

        #[test]
        fn the_derive_lists_editable_fields_only() {
            let schema = Card::schema();
            assert_eq!(Card::COMPONENT, "Card");
            let names: Vec<&str> = schema.fields.iter().map(|f| f.name.as_str()).collect();
            assert_eq!(
                names,
                ["title", "x", "count", "fill", "visible", "align", "opacity"]
            );
            let x = &schema.fields[1];
            assert_eq!(
                (x.min, x.max, x.unit.as_deref()),
                (Some(0.0), Some(1920.0), Some("px"))
            );
            assert_eq!(schema.fields[2].kind, PropKind::Integer);
            assert_eq!(schema.fields[3].kind, PropKind::Color);
            assert_eq!(
                schema.fields[5].kind,
                PropKind::Choice {
                    options: vec!["Left".into(), "Right".into()]
                }
            );
            let values = card().values();
            assert_eq!(values["title"], "Ada");
            assert_eq!(values["fill"], "#ff0000");
            assert_eq!(values["align"], "Left");
            assert_eq!(values.len(), 7);
        }

        #[test]
        fn apply_sets_what_fits_and_names_the_rest() {
            let mut card = card();
            let values: PropValues = [
                ("title", Value::from("Grace")),
                ("x", Value::from(64.0)),
                ("fill", Value::from("#00ff0080")),
                ("align", Value::from("Right")),
                ("opacity", Value::from(1.0)),
                ("count", Value::from(-3)),
                ("hidden", Value::from(5.0)),
                ("children", Value::from(1)),
                ("nope", Value::from(1)),
            ]
            .into_iter()
            .map(|(k, v)| (k.to_owned(), v))
            .collect();
            let mut rejected = card.apply(&values);
            rejected.sort();
            assert_eq!(rejected, ["children", "count", "hidden", "nope"]);
            assert_eq!(card.title, "Grace");
            assert_eq!(card.x, 64.0);
            assert_eq!(card.fill, Color::rgba(0, 255, 0, 128));
            assert_eq!(card.align, Align::Right);
            assert!(matches!(card.opacity, Anim::Fixed(v) if v == 1.0));
            assert_eq!((card.count, card.hidden), (2, 1.0));
            let out_of_range: PropValues = [("x".to_owned(), Value::from(5000.0))].into();
            assert_eq!(card.apply(&out_of_range), ["x"]);
            assert_eq!(card.x, 64.0);
        }

        #[test]
        fn the_store_swaps_documents_whole() {
            let store = PropsStore::default();
            let identity = EditorObjectIdentity {
                scene_instance_key: "s".into(),
                component_key: "C".into(),
                object_key: "o".into(),
                repeat_key: REPEAT_KEY.into(),
            };
            let before = store.document();
            let mut doc = PropsDocument::default();
            doc.set(&identity, "x", Some(Value::from(1)));
            store.set(Some(doc.clone()));
            assert_eq!(*store.document(), doc);
            assert_eq!(
                *before,
                PropsDocument::default(),
                "snapshots keep their document"
            );
            store.set(None);
            assert_eq!(*store.document(), PropsDocument::default());
        }
    }
}
