//! Editable components in a real render: a props source changes what they draw, names the
//! `<g>` they are drawn in, and hears how they rendered while it listens.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use components_gallery::GalleryMedia;
use fframes::{Previewer, RenderOptions, StaticMediaProvider};
use fframes_kit::prelude::*;
use fframes_kit::props::{EditableObject, PropValues, PropsInfo};
use serde_json::json;

/// Values and records keyed by `component/key`, like an editor's store.
#[derive(Debug, Default)]
struct Store {
    values: Mutex<HashMap<String, PropValues>>,
    rendered: Mutex<HashMap<String, PropsInfo>>,
}

fn label(object: EditableObject<'_>) -> String {
    format!("{}/{}", object.component, object.key)
}

impl Store {
    fn set(&self, object: &str, values: PropValues) {
        self.values.lock().unwrap().insert(object.into(), values);
    }

    fn info(&self, object: &str) -> PropsInfo {
        self.rendered.lock().unwrap()[object].clone()
    }
}

/// One snapshot of the store's values, made per `Ui`.
struct View<'a> {
    store: &'a Store,
    values: HashMap<String, PropValues>,
}

impl PropsSource for View<'_> {
    fn values(&self, object: EditableObject<'_>) -> Option<&PropValues> {
        self.values.get(&label(object))
    }

    fn listening(&self) -> bool {
        true
    }

    fn rendered(&self, object: EditableObject<'_>, info: PropsInfo) {
        self.store
            .rendered
            .lock()
            .unwrap()
            .insert(label(object), info);
    }

    fn element_id(&self, object: EditableObject<'_>) -> Option<String> {
        Some(format!("edit-{}-{}", object.component, object.key))
    }
}

/// A lower third marked editable in struct form and a circle marked in tag form; `store`
/// decides whether the `Ui` renders with props.
#[derive(Debug)]
struct Probe {
    theme: Theme,
    store: Option<Arc<Store>>,
    editable: bool,
    drawn: Mutex<String>,
}

impl Probe {
    fn new(store: Option<Arc<Store>>, editable: bool) -> Self {
        Self {
            theme: Theme::default(),
            store,
            editable,
            drawn: Mutex::new(String::new()),
        }
    }
}

impl Video for Probe {
    const FPS: usize = 30;
    const WIDTH: usize = 640;
    const HEIGHT: usize = 360;

    fn duration(&self) -> Duration<'_> {
        Duration::Seconds(1.0)
    }

    fn audio(&self) -> AudioMap<'_> {
        AudioMap::none()
    }

    fn render_frame<'a>(&'a self, frame: Frame, ctx: &FFramesContext<'a, '_>) -> Svgr<'a> {
        let mut ui = Ui::new(frame, ctx, &self.theme);
        if let Some(store) = &self.store {
            let values = store.values.lock().unwrap().clone();
            ui = ui.with_props(View { store, values });
        }
        let lower_third = LowerThird {
            name: "Ada".into(),
            role: "Mathematician".into(),
            ..ui.defaults()
        };
        let tree = if self.editable {
            let lower_third = ui.show(lower_third.editable("speaker"));
            let circle = ui!(<Circle editable="dot" cx={320.0} cy={180.0} radius={40.0} />);
            svgr!(<g>{lower_third}{circle}</g>)
        } else {
            let lower_third = ui.show(lower_third);
            let circle = ui!(<Circle cx={320.0} cy={180.0} radius={40.0} />);
            svgr!(<g>{lower_third}{circle}</g>)
        };
        *self.drawn.lock().unwrap() = tree.to_string();
        svgr!(<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 640 360" width="640" height="360">{tree}</svg>)
    }
}

fn drawn(probe: &Probe, frame: usize) -> String {
    let media = GalleryMedia::prepare().expect("gallery media");
    let options = RenderOptions {
        media: Some(&media),
        ..Default::default()
    };
    let mut previewer = Previewer::new(probe, &options).expect("previewer");
    previewer.svg(frame).expect("frame");
    probe.drawn.lock().unwrap().clone()
}

#[test]
fn without_a_source_editable_components_draw_unchanged() {
    assert_eq!(
        drawn(&Probe::new(None, true), 20),
        drawn(&Probe::new(None, false), 20)
    );
}

#[test]
fn a_source_changes_what_editable_components_draw() {
    let store = Arc::new(Store::default());
    let probe = Probe::new(Some(store.clone()), true);
    let before = drawn(&probe, 20);
    assert!(before.contains("Ada"), "{before}");
    // the source names the `<g>` of each component
    assert!(before.contains("edit-LowerThird-speaker"), "{before}");
    assert!(before.contains("edit-Circle-dot"), "{before}");

    let info = store.info("LowerThird/speaker");
    assert_eq!(info.code_values["name"], "Ada");
    assert!(info.schema.fields.iter().any(|f| f.name == "role"));

    store.set(
        "LowerThird/speaker",
        [("name".to_owned(), json!("Grace"))].into(),
    );
    store.set("Circle/dot", [("radius".to_owned(), json!(90.0))].into());
    let after = drawn(&probe, 20);
    assert!(after.contains("Grace") && !after.contains("Ada"), "{after}");
    let info = store.info("LowerThird/speaker");
    assert_eq!(info.code_values["name"], "Ada");
    assert_eq!(info.values["name"], "Grace");
    assert_eq!(store.info("Circle/dot").values["radius"], 90.0);

    store.values.lock().unwrap().clear();
    assert_eq!(drawn(&probe, 20), before);
}
