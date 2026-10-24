//! The `ui!` macro and the `Props` derives of `fframes-kit`, re-exported as
//! `fframes_kit::ui` and in `fframes_kit::props`. See their docs there.

mod props;
mod tags;

/// Makes a props struct editable in fframes Studio: implements `fframes_kit::props::Props`.
/// Fields whose type implements `PropValue` (numbers, `bool`, `String`, `Cow<str>`,
/// `Color`, `Anim<f32>`, `#[derive(PropChoice)]` enums) are editable; the others are left
/// out without an attribute. `#[props(skip)]` leaves an editable field out, and
/// `#[props(min = 0.0, max = 100.0, step = 1.0, unit = "px")]` bounds a number.
#[proc_macro_derive(Props, attributes(props))]
pub fn derive_props(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    props::derive_props(input.into()).into()
}

/// Makes an enum whose variants have no fields an editable prop: Studio shows the
/// variant names as a choice.
#[proc_macro_derive(PropChoice)]
pub fn derive_prop_choice(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    props::derive_choice(input.into()).into()
}

/// `svgr!` markup with component tags: `<LowerThird name="Ada" role="Mathematician" />`
/// becomes `{ui.show(LowerThird { name: "Ada".into(), role: "Mathematician".into(),
/// ..ui.defaults() })}`, and everything else goes to `fframes::svgr!` unchanged.
///
/// - A component tag is one identifier that starts with an uppercase letter; SVG element
///   names start lowercase.
/// - Attributes are fields. A quoted value or an `{expr}` goes through `.into()`; a number
///   or `true`/`false` alone in braces (`{0.5}`, `{3}`) is written as is, so the field's type
///   decides it. A bare attribute is `true`. Dashed names are an error naming the field
///   (`font-size` is `font_size`).
/// - Children become `children: fframes_kit::ui::children(move |ui| ui!(<g>..</g>))`, so
///   they can hold component tags too and see the container's `ui` (its local time). The
///   closure is `Fn` and runs every frame, so build what the children draw inside them; an
///   owned `Svgr` from outside cannot be moved in.
///
/// Component tags need a `ui: Ui` in scope; create one with
/// `let mut ui = Ui::new(frame, ctx, &self.theme)`. Without it the compiler reports
/// "cannot find value `ui`" at the first tag. `docs/components/components.md` has
/// examples.
#[proc_macro]
pub fn ui(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    tags::expand(input.into()).into()
}
