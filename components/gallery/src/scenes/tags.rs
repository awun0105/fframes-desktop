//! `SquareToCircle` and `FadeIn` again, written with component tags in `ui!`. Their frames
//! must equal the struct-form scenes' frames.

use fframes_kit::prelude::*;

use super::animate::{CX, CY, circle, square};
use super::core::{fade_in_cards, title};
use crate::GalleryScene;

/// The square morphing into the circle, then fading out.
fn morph<'a>(ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
    let morph = ui.show(Morph::new(square(ui).shape(), circle(ui).shape()));
    FadeOut {
        run_time: 0.8,
        ..ui.defaults()
    }
    .apply(ui, morph)
}

fn square_to_circle<'a>(ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
    let create: Option<EnterAnim> = Some(Create::default().into());
    ui!(
        <g>
            {title(ui, "SquareToCircle")}
            <g>
                <Sequence duration={Some(1.2)}>
                    <Square cx={CX} cy={CY} side_length={400.0} enter={create} />
                </Sequence>
                <Sequence from={1.2}>
                    {morph(ui)}
                </Sequence>
            </g>
        </g>
    )
}

fn fade_in<'a>(ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
    ui!(
        <g>
            {title(ui, "FadeIn")}
            <Fill>{fade_in_cards(ui)}</Fill>
        </g>
    )
}

pub fn scenes() -> Vec<GalleryScene> {
    vec![
        GalleryScene::new("TagsSquareToCircle", 4.0, square_to_circle),
        GalleryScene::new("TagsFadeIn", 2.0, fade_in),
    ]
}
