//! `#[derive(Props)]` and `#[derive(PropChoice)]`, expanded against `fframes_kit::props`.

use proc_macro2::TokenStream;
use quote::quote;
use syn::{Data, DeriveInput, Expr, Fields, LitStr, Type, spanned::Spanned};

/// `#[props(..)]` on a field.
#[derive(Default)]
struct FieldOptions {
    skip: bool,
    min: Option<Expr>,
    max: Option<Expr>,
    step: Option<Expr>,
    unit: Option<LitStr>,
}

fn field_options(attrs: &[syn::Attribute]) -> syn::Result<FieldOptions> {
    let mut options = FieldOptions::default();
    for attr in attrs.iter().filter(|attr| attr.path().is_ident("props")) {
        attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("skip") {
                options.skip = true;
            } else if meta.path.is_ident("min") {
                options.min = Some(meta.value()?.parse()?);
            } else if meta.path.is_ident("max") {
                options.max = Some(meta.value()?.parse()?);
            } else if meta.path.is_ident("step") {
                options.step = Some(meta.value()?.parse()?);
            } else if meta.path.is_ident("unit") {
                options.unit = Some(meta.value()?.parse()?);
            } else {
                return Err(meta.error("expected `skip`, `min`, `max`, `step` or `unit`"));
            }
            Ok(())
        })?;
    }
    Ok(options)
}

fn option(value: Option<&Expr>) -> TokenStream {
    if let Some(value) = value {
        quote!(::core::option::Option::Some(f64::from(#value)))
    } else {
        quote!(::core::option::Option::None)
    }
}

pub fn derive_props(input: TokenStream) -> TokenStream {
    match props(input) {
        Ok(tokens) => tokens,
        Err(error) => error.to_compile_error(),
    }
}

fn props(input: TokenStream) -> syn::Result<TokenStream> {
    let input: DeriveInput = syn::parse2(input)?;
    let Data::Struct(data) = &input.data else {
        return Err(syn::Error::new(
            input.span(),
            "`Props` is derived for structs with named fields",
        ));
    };
    let Fields::Named(named) = &data.fields else {
        return Err(syn::Error::new(
            input.span(),
            "`Props` is derived for structs with named fields",
        ));
    };
    let name = &input.ident;
    let component = name.to_string();
    let (impl_generics, type_generics, where_clause) = input.generics.split_for_impl();
    let private = quote!(::fframes_kit::props::__private);

    let mut schema = Vec::new();
    let mut values = Vec::new();
    let mut apply = Vec::new();
    for field in &named.named {
        let options = field_options(&field.attrs)?;
        if options.skip {
            continue;
        }
        let ident = field.ident.as_ref().expect("named fields");
        let key = ident.to_string();
        let ty: &Type = &field.ty;
        let (min, max, step) = (
            option(options.min.as_ref()),
            option(options.max.as_ref()),
            option(options.step.as_ref()),
        );
        let unit = if let Some(unit) = &options.unit {
            quote!(::core::option::Option::Some(::std::string::String::from(#unit)))
        } else {
            quote!(::core::option::Option::None)
        };
        schema.push(quote! {
            if let ::core::option::Option::Some(kind) =
                (&#private::Kind::<#ty>(::core::marker::PhantomData)).__kind()
            {
                fields.push(::fframes_kit::props::PropField {
                    name: ::std::string::String::from(#key),
                    kind,
                    min: #min,
                    max: #max,
                    step: #step,
                    unit: #unit,
                });
            }
        });
        values.push(quote! {
            if let ::core::option::Option::Some(value) = (&#private::Slot::get(&self.#ident)).__get() {
                values.insert(::std::string::String::from(#key), value);
            }
        });
        let in_range = if options.min.is_none() && options.max.is_none() {
            quote!(true)
        } else {
            quote!(#private::in_range(value, #min, #max))
        };
        apply.push(quote! {
            #key => #in_range && (&#private::Slot::set(&mut self.#ident)).__set(value),
        });
    }

    Ok(quote! {
        #[allow(clippy::all, clippy::pedantic)]
        impl #impl_generics ::fframes_kit::props::Props for #name #type_generics #where_clause {
            const COMPONENT: &'static str = #component;

            fn schema() -> ::fframes_kit::props::PropsSchema {
                #[allow(unused_imports)]
                use #private::{EditableKind as _, NotEditableKind as _};
                let mut fields = ::std::vec::Vec::new();
                #(#schema)*
                ::fframes_kit::props::PropsSchema {
                    component: ::std::string::String::from(#component),
                    fields,
                }
            }

            fn values(&self) -> ::fframes_kit::props::PropValues {
                #[allow(unused_imports)]
                use #private::{Editable as _, NotEditable as _};
                let mut values = ::fframes_kit::props::PropValues::new();
                #(#values)*
                values
            }

            fn apply(
                &mut self,
                values: &::fframes_kit::props::PropValues,
            ) -> ::std::vec::Vec<::std::string::String> {
                #[allow(unused_imports)]
                use #private::{Editable as _, NotEditable as _};
                let mut rejected = ::std::vec::Vec::new();
                for (name, value) in values {
                    let applied = match name.as_str() {
                        #(#apply)*
                        _ => false,
                    };
                    if !applied {
                        rejected.push(name.clone());
                    }
                }
                rejected
            }
        }
    })
}

pub fn derive_choice(input: TokenStream) -> TokenStream {
    match choice(input) {
        Ok(tokens) => tokens,
        Err(error) => error.to_compile_error(),
    }
}

fn choice(input: TokenStream) -> syn::Result<TokenStream> {
    let input: DeriveInput = syn::parse2(input)?;
    let Data::Enum(data) = &input.data else {
        return Err(syn::Error::new(
            input.span(),
            "`PropChoice` is derived for enums whose variants have no fields",
        ));
    };
    let name = &input.ident;
    let (impl_generics, type_generics, where_clause) = input.generics.split_for_impl();
    let mut variants = Vec::new();
    for variant in &data.variants {
        if !matches!(variant.fields, Fields::Unit) {
            return Err(syn::Error::new(
                variant.span(),
                "`PropChoice` variants cannot have fields",
            ));
        }
        variants.push((&variant.ident, variant.ident.to_string()));
    }
    let options = variants.iter().map(|(_, label)| label);
    let to_value = variants
        .iter()
        .map(|(ident, label)| quote!(#name::#ident => #label,));
    let set_value = variants
        .iter()
        .map(|(ident, label)| quote!(::core::option::Option::Some(#label) => #name::#ident,));
    let private = quote!(::fframes_kit::props::__private);
    Ok(quote! {
        #[allow(clippy::all, clippy::pedantic)]
        impl #impl_generics ::fframes_kit::props::PropValue for #name #type_generics #where_clause {
            fn kind() -> ::fframes_kit::props::PropKind {
                ::fframes_kit::props::PropKind::Choice {
                    options: ::std::vec![#(::std::string::String::from(#options)),*],
                }
            }
            fn to_value(&self) -> ::core::option::Option<#private::Value> {
                ::core::option::Option::Some(#private::choice_value(match self {
                    #(#to_value)*
                }))
            }
            fn set_value(&mut self, value: &#private::Value) -> bool {
                *self = match value.as_str() {
                    #(#set_value)*
                    _ => return false,
                };
                true
            }
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn squeeze(tokens: &TokenStream) -> String {
        tokens.to_string().split_whitespace().collect()
    }

    #[test]
    fn every_field_is_probed_and_skip_leaves_one_out() {
        let out = derive_props(quote! {
            struct Card<'a> {
                title: Cow<'a, str>,
                #[props(min = 0.0, max = 1920.0, unit = "px")]
                x: f32,
                #[props(skip)]
                secret: f32,
                children: Children<'a>,
            }
        });
        let text = squeeze(&out);
        assert!(
            text.contains("impl<'a>::fframes_kit::props::PropsforCard<'a>"),
            "{text}"
        );
        assert!(text.contains("constCOMPONENT:&'staticstr=\"Card\""));
        for field in ["title", "x", "children"] {
            assert!(text.contains(&format!("\"{field}\"=>")), "{field}");
        }
        assert!(!text.contains("secret"));
        assert!(text.contains("in_range(value,::core::option::Option::Some(f64::from(0.0))"));
        assert!(text.contains("String::from(\"px\")"));
    }

    #[test]
    fn options_and_shapes_are_checked() {
        for (input, message) in [
            (
                quote!(
                    struct A {
                        #[props(color)]
                        x: f32,
                    }
                ),
                "expected `skip`",
            ),
            (
                quote!(
                    struct A(f32);
                ),
                "named fields",
            ),
            (
                quote!(
                    enum A {
                        B,
                    }
                ),
                "named fields",
            ),
        ] {
            let text = derive_props(input).to_string();
            assert!(text.contains("compile_error"), "{text}");
            assert!(text.contains(message), "{text}");
        }
        let text = derive_choice(quote!(
            enum A {
                B(u8),
            }
        ))
        .to_string();
        assert!(text.contains("cannot have fields"), "{text}");
    }

    #[test]
    fn choices_name_their_variants() {
        let text = squeeze(&derive_choice(quote!(
            enum Align {
                Left,
                Right,
            }
        )));
        assert!(text.contains("options:::std::vec![::std::string::String::from(\"Left\"),::std::string::String::from(\"Right\")]"), "{text}");
        assert!(text.contains("Align::Left=>\"Left\""));
        assert!(text.contains("::core::option::Option::Some(\"Right\")=>Align::Right"));
    }
}
