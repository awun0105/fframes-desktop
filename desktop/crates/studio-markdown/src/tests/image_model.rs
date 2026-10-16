use crate::parse::*;
use crate::*;

#[test]
fn images_preserve_alt_title_position_links_and_empty_alt() {
    let tree =
        parse_full("Before [![**alt**](a.png \"Title\")](next.md) after ![](b.png) ![](b.png)");
    let Block::Paragraph { runs } = &tree.blocks[0].block else {
        panic!("paragraph");
    };
    let images: Vec<_> = runs.iter().filter_map(|r| r.style.image.as_ref()).collect();
    assert_eq!(images.len(), 3);
    assert_eq!(images[0].alt, "alt");
    assert_eq!(images[0].title, "Title");
    assert_eq!(images[0].link.as_deref(), Some("next.md"));
    assert_eq!(images[1].source, "b.png");
    assert!(images[1].alt.is_empty());
    assert_eq!(runs.first().unwrap().text, "Before ");
}
