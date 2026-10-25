//! Editable props: components that fframes Studio can change without editing code.
//!
//! A component becomes editable with `#[derive(Props)]` (feature `studio`) and a key:
//! `ui.show(LowerThird { .. }.editable("speaker"))`, or `<LowerThird editable="speaker" .. />`
//! in [`ui!`](crate::ui!). The values of the `Ui`'s [`PropsSource`] then replace the code's
//! values when the component renders, and the component is drawn inside a `<g>` whose id
//! the source gives. Without a source, or without the `studio` feature, `editable` renders
//! the component unchanged.
//!
//! The kit does not know Studio. fframes Studio's source is the `PropsStore` of the
//! `fframes-studio-kit` crate, which holds the project's `props.json`:
//!
//! ```ignore
//! let mut ui = Ui::new(frame, ctx, &self.theme).with_props(self.store.view());
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
    use std::collections::BTreeMap;

    pub use fframes_kit_macros::{PropChoice, Props};
    use serde_json::Value;

    use fframes::diagnostics::{Diagnostic, report};
    use fframes::{Color, Svgr};

    use super::Editable;
    use crate::anim::Anim;
    use crate::ui::{Component, Ui};

    /// Field values as JSON, keyed by field name.
    pub type PropValues = BTreeMap<String, Value>;

    /// What a field holds, which decides the control an editor shows for it.
    #[derive(Debug, Clone, PartialEq)]
    pub enum PropKind {
        /// A JSON number.
        Number,
        /// A JSON integer.
        Integer,
        /// A JSON string.
        Text,
        /// A `#rrggbb` or `#rrggbbaa` string.
        Color,
        /// A JSON boolean.
        Toggle,
        /// One of `options`, as a string.
        Choice { options: Vec<String> },
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct PropField {
        pub name: String,
        pub kind: PropKind,
        pub min: Option<f64>,
        pub max: Option<f64>,
        pub step: Option<f64>,
        pub unit: Option<String>,
    }

    /// The editable fields of a component type.
    #[derive(Debug, Clone, PartialEq)]
    pub struct PropsSchema {
        pub component: String,
        pub fields: Vec<PropField>,
    }

    /// An editable object as it rendered: its schema, the values its code gave it and the
    /// values it drew with after the source's overrides.
    #[derive(Debug, Clone, PartialEq)]
    pub struct PropsInfo {
        pub schema: PropsSchema,
        pub code_values: PropValues,
        pub values: PropValues,
    }

    /// One editable object, as a [`PropsSource`] sees it.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct EditableObject<'k> {
        /// The component's type name, [`Props::COMPONENT`].
        pub component: &'static str,
        /// The key given to `editable`.
        pub key: &'k str,
    }

    /// Where editable components take their values from, and where they tell how they
    /// rendered. Set on a `Ui` with [`Ui::with_props`]; its `Ui`s inside containers share it.
    ///
    /// A source is made per `Ui`, so it can hold one snapshot of its values: values that
    /// change later apply from the next `Ui`.
    pub trait PropsSource: Send + Sync {
        /// The values that replace the code's values of `object`.
        fn values(&self, object: EditableObject<'_>) -> Option<&PropValues>;

        /// Whether [`rendered`](Self::rendered) wants to hear about objects; the code's
        /// values are only collected then.
        fn listening(&self) -> bool {
            false
        }

        /// How `object` rendered, while [`listening`](Self::listening).
        fn rendered(&self, object: EditableObject<'_>, info: PropsInfo) {
            let _ = (object, info);
        }

        /// The id of the `<g>` the object is drawn in; `None` draws it without one.
        fn element_id(&self, object: EditableObject<'_>) -> Option<String> {
            let _ = object;
            None
        }
    }

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

    impl<'a, C: Component<'a> + Props> Component<'a> for Editable<'a, C> {
        fn render(mut self, ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
            let Some(source) = ui.props.clone() else {
                return ui.show(self.component);
            };
            let object = EditableObject {
                component: C::COMPONENT,
                key: &self.key,
            };
            let code_values = source.listening().then(|| self.component.values());
            if let Some(values) = source.values(object) {
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
                source.rendered(object, info);
            }
            let id = source.element_id(object);
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
    }
}
