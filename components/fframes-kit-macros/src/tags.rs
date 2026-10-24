//! The token rewrite behind `ui!`: component tags become `{ui.show(..)}` blocks, every other
//! token is copied through.

use proc_macro2::{Delimiter, Group, Ident, Literal, Span, TokenStream, TokenTree};
use quote::{quote, quote_spanned};

/// A rewrite error and the token it is about.
#[derive(Debug)]
pub struct Error {
    span: Span,
    message: String,
}

impl Error {
    fn new(span: Span, message: impl Into<String>) -> Self {
        Self {
            span,
            message: message.into(),
        }
    }

    fn to_compile_error(&self) -> TokenStream {
        let message = &self.message;
        quote_spanned!(self.span=> ::core::compile_error!(#message))
    }
}

/// The macro: the rewritten markup in `fframes::svgr!`, or the `{ui.show(..)}` block alone
/// when the markup is one component tag.
pub fn expand(input: TokenStream) -> TokenStream {
    let tokens: Vec<TokenTree> = input.into_iter().collect();
    let single = match tokens.as_slice() {
        [TokenTree::Punct(p), TokenTree::Ident(name), ..] if p.as_char() == '<' => {
            is_component(name)
        }
        _ => false,
    };
    match rewrite_tokens(&tokens) {
        Ok(out) => {
            let mut trees = out.clone().into_iter();
            if let (true, Some(block @ TokenTree::Group(_)), None) =
                (single, trees.next(), trees.next())
            {
                block.into()
            } else {
                quote!(::fframes::svgr!(#out))
            }
        }
        Err(error) => error.to_compile_error(),
    }
}

fn is_component(name: &Ident) -> bool {
    name.to_string()
        .starts_with(|c: char| c.is_ascii_uppercase())
}

fn is_punct(tree: Option<&TokenTree>, c: char) -> bool {
    matches!(tree, Some(TokenTree::Punct(p)) if p.as_char() == c)
}

fn rewrite_tokens(tokens: &[TokenTree]) -> Result<TokenStream, Error> {
    let mut out = TokenStream::new();
    let mut i = 0;
    while i < tokens.len() {
        if is_punct(tokens.get(i), '<')
            && let Some(TokenTree::Ident(name)) = tokens.get(i + 1)
            && is_component(name)
        {
            let (block, next) = component(tokens, i, name)?;
            out.extend([block]);
            i = next;
            continue;
        }
        if is_punct(tokens.get(i), '<')
            && is_punct(tokens.get(i + 1), '/')
            && let Some(TokenTree::Ident(name)) = tokens.get(i + 2)
            && is_component(name)
        {
            return Err(Error::new(
                name.span(),
                format!("`</{name}>` closes a tag that was not opened"),
            ));
        }
        out.extend([tokens[i].clone()]);
        i += 1;
    }
    Ok(out)
}

/// A field from an attribute: `name: value`.
struct Field {
    name: Ident,
    value: TokenStream,
}

/// The component tag that starts at `tokens[start]` (its `<`): the `{ui.show(..)}` block and
/// the index after the tag.
fn component(
    tokens: &[TokenTree],
    start: usize,
    name: &Ident,
) -> Result<(TokenTree, usize), Error> {
    let mut i = start + 2;
    let mut fields = Vec::new();
    let mut editable = None;
    let self_closing = loop {
        match tokens.get(i) {
            Some(TokenTree::Punct(p)) if p.as_char() == '/' && is_punct(tokens.get(i + 1), '>') => {
                i += 2;
                break true;
            }
            Some(TokenTree::Punct(p)) if p.as_char() == '>' => {
                i += 1;
                break false;
            }
            Some(TokenTree::Ident(attr)) if attr == "editable" => {
                // the editor key, not a field: `editable="speaker"` or `editable={key}`
                let key = match (tokens.get(i + 1), tokens.get(i + 2)) {
                    (Some(TokenTree::Punct(p)), Some(TokenTree::Literal(lit)))
                        if p.as_char() == '=' && is_string(lit) =>
                    {
                        quote!(#lit)
                    }
                    (Some(TokenTree::Punct(p)), Some(TokenTree::Group(group)))
                        if p.as_char() == '=' && group.delimiter() == Delimiter::Brace =>
                    {
                        group.stream()
                    }
                    _ => {
                        return Err(Error::new(
                            attr.span(),
                            "`editable` needs a key: `editable=\"speaker\"` or `editable={key}`",
                        ));
                    }
                };
                editable = Some(key);
                i += 3;
            }
            Some(TokenTree::Ident(attr)) => {
                let (field, next) = attribute(tokens, i, attr)?;
                fields.push(field);
                i = next;
            }
            Some(other) => {
                return Err(Error::new(
                    other.span(),
                    format!("expected an attribute of `{name}`, `/>` or `>`"),
                ));
            }
            None => return Err(Error::new(name.span(), format!("`<{name}` is not closed"))),
        }
    };

    let ui = Ident::new("ui", name.span());
    if !self_closing {
        let (children, next) = children(tokens, i, name)?;
        i = next;
        if !children.is_empty() {
            let field = Ident::new("children", name.span());
            fields.push(Field {
                value: quote_spanned!(name.span()=>
                    ::fframes_kit::ui::children(move |#ui| ::fframes_kit::ui!(<g>#children</g>))
                ),
                name: field,
            });
        }
    }

    let fields = fields
        .into_iter()
        .map(|Field { name, value }| quote!(#name: #value,));
    // a binding of the macro's own, so it cannot shadow or be shadowed by the caller's names;
    // the struct update has no effect when every field is set, which is fine here
    let props = Ident::new("props", Span::mixed_site());
    let shown = if let Some(key) = editable {
        quote_spanned!(name.span()=> ::fframes_kit::props::Editable::new(#key, #props))
    } else {
        quote!(#props)
    };
    let call = quote_spanned!(name.span()=>
        #[allow(clippy::needless_update)]
        let #props = #name { #(#fields)* ..#ui.defaults() };
        #ui.show(#shown)
    );
    let mut block = Group::new(Delimiter::Brace, call);
    block.set_span(name.span());
    Ok((TokenTree::Group(block), i))
}

/// The attribute that starts with `attr` at `tokens[start]` and the index after it.
fn attribute(tokens: &[TokenTree], start: usize, attr: &Ident) -> Result<(Field, usize), Error> {
    if is_punct(tokens.get(start + 1), '-') {
        let mut dashed = attr.to_string();
        let mut i = start + 1;
        while is_punct(tokens.get(i), '-')
            && let Some(TokenTree::Ident(part)) = tokens.get(i + 1)
        {
            dashed.push('-');
            dashed.push_str(&part.to_string());
            i += 2;
        }
        return Err(Error::new(
            attr.span(),
            format!(
                "`{dashed}` is not a field name; component attributes are Rust fields, write `{}`",
                dashed.replace('-', "_")
            ),
        ));
    }
    if !is_punct(tokens.get(start + 1), '=') {
        let value = quote_spanned!(attr.span()=> true);
        return Ok((
            Field {
                name: attr.clone(),
                value,
            },
            start + 1,
        ));
    }
    let value = match tokens.get(start + 2) {
        Some(TokenTree::Literal(lit)) => literal_value(lit),
        Some(TokenTree::Group(group)) if group.delimiter() == Delimiter::Brace => {
            block_value(group)
        }
        Some(other) => {
            return Err(Error::new(
                other.span(),
                format!("the value of `{attr}` must be a literal or an `{{expr}}` block"),
            ));
        }
        None => {
            return Err(Error::new(attr.span(), format!("`{attr}=` needs a value")));
        }
    };
    Ok((
        Field {
            name: attr.clone(),
            value,
        },
        start + 3,
    ))
}

/// A quoted value goes through `.into()`; a number is written as is.
fn literal_value(lit: &Literal) -> TokenStream {
    if is_string(lit) {
        quote_spanned!(lit.span()=> ::fframes_kit::ui::__prop(#lit))
    } else {
        quote!(#lit)
    }
}

/// `{3}`, `{0.5}`, `{true}` are written as is; any other block goes through `.into()`.
fn block_value(group: &Group) -> TokenStream {
    let inner: Vec<TokenTree> = group.stream().into_iter().collect();
    match inner.as_slice() {
        [TokenTree::Literal(lit)] => literal_value(lit),
        [TokenTree::Ident(word)] if word == "true" || word == "false" => quote!(#word),
        _ => {
            let expr = group.stream();
            quote_spanned!(group.span()=> ::fframes_kit::ui::__prop(#expr))
        }
    }
}

fn is_string(lit: &Literal) -> bool {
    let text = lit.to_string();
    text.starts_with('"') || text.starts_with("r\"") || text.starts_with("r#")
}

/// The children of `<name ..>` from `tokens[start]` up to its `</name>`, and the index
/// after the closing tag. Tags of the same name nest.
fn children(
    tokens: &[TokenTree],
    start: usize,
    name: &Ident,
) -> Result<(TokenStream, usize), Error> {
    let mut depth = 0usize;
    let mut i = start;
    while i < tokens.len() {
        let opens = is_punct(tokens.get(i), '<')
            && matches!(tokens.get(i + 1), Some(TokenTree::Ident(n)) if n == name);
        let closes = is_punct(tokens.get(i), '<')
            && is_punct(tokens.get(i + 1), '/')
            && matches!(tokens.get(i + 2), Some(TokenTree::Ident(n)) if n == name);
        if opens && !self_closes(tokens, i + 2) {
            depth += 1;
        } else if closes {
            if depth == 0 {
                if !is_punct(tokens.get(i + 3), '>') {
                    return Err(Error::new(name.span(), format!("`</{name}` needs a `>`")));
                }
                let children = tokens[start..i].iter().cloned().collect();
                return Ok((children, i + 4));
            }
            depth -= 1;
        }
        i += 1;
    }
    Err(Error::new(
        name.span(),
        format!("`<{name}>` has no closing `</{name}>`"),
    ))
}

/// Whether the tag whose attributes start at `tokens[start]` ends with `/>`.
fn self_closes(tokens: &[TokenTree], start: usize) -> bool {
    let mut i = start;
    while let Some(tree) = tokens.get(i) {
        if is_punct(Some(tree), '/') && is_punct(tokens.get(i + 1), '>') {
            return true;
        }
        if is_punct(Some(tree), '>') {
            return false;
        }
        i += 1;
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The markup with its component tags replaced, as `svgr!` gets it.
    fn rewrite(input: TokenStream) -> Result<TokenStream, Error> {
        rewrite_tokens(&input.into_iter().collect::<Vec<_>>())
    }

    fn rewritten(input: &str) -> String {
        rewrite(input.parse().unwrap()).unwrap().to_string()
    }

    /// The same tokens; punctuation spacing (`Joint` or `Alone`) may differ in print.
    fn same(a: &str, b: &str) {
        let b: TokenStream = b.parse().unwrap();
        let squeeze = |s: String| s.split_whitespace().collect::<String>();
        assert_eq!(squeeze(rewritten(a)), squeeze(b.to_string()));
    }

    /// What a tag becomes: its props built from `fields`, then shown.
    fn show(name: &str, fields: &str) -> String {
        format!(
            "{{ #[allow(clippy::needless_update)] let props = {name} {{ {fields} ..ui.defaults() }}; \
             ui.show(props) }}"
        )
    }

    const PROP: &str = "::fframes_kit::ui::__prop";

    fn error(input: &str) -> String {
        rewrite(input.parse().unwrap()).unwrap_err().message
    }

    #[test]
    fn svg_only_markup_passes_through_unchanged() {
        let input = r##"<svg viewBox="0 0 10 10" width={W}>
            <g transform={t}><rect x="1" fill="#fff" /><text font-size="20">"hi " {name}</text></g>
            {extra}
        </svg>"##;
        let tokens: TokenStream = input.parse().unwrap();
        assert_eq!(rewritten(input), tokens.to_string());
    }

    #[test]
    fn editable_wraps_the_props_and_is_not_a_field() {
        let wrapped = |key: &str| {
            format!(
                "{{ #[allow(clippy::needless_update)] let props = LowerThird {{ name: {PROP}(\"Ada\"), ..ui.defaults() }}; \
                 ui.show(::fframes_kit::props::Editable::new({key}, props)) }}"
            )
        };
        same(
            r#"<LowerThird editable="speaker" name="Ada" />"#,
            &wrapped("\"speaker\""),
        );
        same(
            r#"<LowerThird name="Ada" editable={key} />"#,
            &wrapped("key"),
        );
        for bad in [
            "<LowerThird editable />",
            "<LowerThird editable={1} editable=3 />",
        ] {
            assert!(error(bad).contains("`editable` needs a key"), "{bad}");
        }
    }

    #[test]
    fn a_tag_becomes_a_show_call() {
        same(
            r#"<LowerThird name="Ada" role="Mathematician" />"#,
            &show(
                "LowerThird",
                &format!(r#"name: {PROP}("Ada"), role: {PROP}("Mathematician"),"#),
            ),
        );
    }

    #[test]
    fn expressions_go_through_into_and_literals_stay() {
        same(
            "<Circle radius={2.0 * r} enter={fade} cx={10.0} count={3} on={false} x=5 />",
            &show(
                "Circle",
                &format!(
                    "radius: {PROP}(2.0 * r), enter: {PROP}(fade), cx: 10.0, count: 3, on: false, x: 5,"
                ),
            ),
        );
    }

    #[test]
    fn a_bare_attribute_is_true() {
        same("<Axes tips />", &show("Axes", "tips: true,"));
    }

    #[test]
    fn dashed_names_name_the_field() {
        let message = error(r#"<Label font-size="20" />"#);
        assert!(
            message.contains("`font-size`") && message.contains("`font_size`"),
            "{message}"
        );
        assert!(error("<Label stroke-dash-array={a} />").contains("`stroke_dash_array`"));
    }

    #[test]
    fn children_are_a_closure_over_the_same_macro() {
        same(
            r#"<Sequence from={1.0}><Circle radius={r} /><rect width="5" /></Sequence>"#,
            &show(
                "Sequence",
                r#"from: 1.0, children: ::fframes_kit::ui::children(move |ui|
                    ::fframes_kit::ui!(<g><Circle radius={r} /><rect width="5" /></g>)),"#,
            ),
        );
        // nested tags of the same name close in order
        let out = rewritten("<Fill><Fill><Dot /></Fill></Fill><rect />");
        let out: String = out.split_whitespace().collect();
        assert!(out.ends_with("ui.show(props)}<rect/>"), "{out}");
        // an empty pair has no children
        same("<Fill></Fill>", &show("Fill", ""));
    }

    #[test]
    fn a_tag_inside_svg_markup_is_replaced_in_place() {
        same(
            r#"<g opacity="0.5"><Dot cx={x} /></g>"#,
            &format!(
                r#"<g opacity="0.5">{}</g>"#,
                show("Dot", &format!("cx: {PROP}(x),"))
            ),
        );
    }

    #[test]
    fn a_single_tag_expands_to_the_call_alone() {
        let out = expand("<Dot />".parse().unwrap()).to_string();
        let expected: TokenStream = show("Dot", "").parse().unwrap();
        let squeeze = |s: String| s.split_whitespace().collect::<String>();
        assert_eq!(squeeze(out), squeeze(expected.to_string()));
        let out = expand("<g><Dot /></g>".parse().unwrap()).to_string();
        assert!(out.starts_with(":: fframes :: svgr !"), "{out}");
    }

    #[test]
    fn broken_tags_are_errors() {
        assert!(error("<Dot cx={1.0}").contains("not closed"));
        assert!(error("<Fill><Dot /></g>").contains("no closing"));
        assert!(error("</Fill>").contains("not opened"));
        assert!(error("<Dot cx=- />").contains("literal or an `{expr}`"));
    }
}
