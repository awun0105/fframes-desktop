//! Numbers, text, coordinate systems, fields and graphs, one scene per group.

use fframes_kit::prelude::*;

use crate::GalleryScene;

pub fn scenes() -> Vec<GalleryScene> {
    vec![
        // the numbers change on every frame, their digits stay in their slots
        GalleryScene::new("DecimalNumber", 3.0, decimal_number),
    ]
}

fn decimal_number<'a>(ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
    let head = ui.show(Title {
        text: "Numbers and text".into(),
        match_underline_width_to_text: true,
        ..ui.defaults()
    });
    // a value that moves on every frame, as a closure of time
    let p = ui.seconds() / 3.0;
    let mono = TextStyle {
        family: "JetBrains Mono",
        ..ui.theme.typography.title
    };
    let money = ui.show(DecimalNumber {
        value: (9_876.54 * p).into(),
        x: 700.0,
        y: 340.0,
        anchor: TextAnchor::End,
        unit: " €",
        ..ui.defaults()
    });
    let signed = ui.show(DecimalNumber {
        value: (4.0 * p - 2.0).into(),
        x: 860.0,
        y: 340.0,
        include_sign: true,
        num_decimal_places: 3,
        style: mono,
        ..ui.defaults()
    });
    let count = ui.show(Integer {
        value: (120.0 * p).into(),
        x: 1720.0,
        y: 340.0,
        anchor: TextAnchor::End,
        ..ui.defaults()
    });
    let variable = ui.show(Variable {
        label: "t".into(),
        value: ui.seconds().into(),
        x: 200.0,
        y: 500.0,
        unit: " s",
        ..ui.defaults()
    });
    let list = ui.show(BulletedList {
        items: vec![
            "Digits sit in fixed slots".into(),
            "Integer rounds".into(),
            "Variable shows label = value".into(),
        ],
        x: 200.0,
        y: 640.0,
        ..ui.defaults()
    });
    let markup = ui.show(MarkupText {
        markup: "Markup: <u>underline</u>, <span foreground='#ff9f43'>colour</span> and <span size='x-large'>size</span>".into(),
        x: 1000.0,
        y: 680.0,
        style: ui.theme.typography.body,
        ..ui.defaults()
    });
    svgr!(<g>{head}{money}{signed}{count}{variable}{list}{markup}</g>)
}
