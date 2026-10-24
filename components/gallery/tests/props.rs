//! Editable components in a real render: a props document changes what they draw, Studio's
//! id names them, and a store that listens records their schema and values.

use std::sync::{Arc, Mutex};

use components_gallery::GalleryMedia;
use fframes::{EditorObjectKey, Previewer, RenderOptions, StaticMediaProvider};
use fframes_kit::prelude::*;
use fframes_kit::props::{EditorObjectIdentity, PropsDocument, REPEAT_KEY};
use serde_json::json;

/// A lower third marked editable in struct form and a circle marked in tag form; `store`
/// decides whether the `Ui` renders with props.
#[derive(Debug)]
struct Probe {
    theme: Theme,
    store: Option<Arc<PropsStore>>,
    editable: bool,
    drawn: Mutex<String>,
}

impl Probe {
    fn new(store: Option<Arc<PropsStore>>, editable: bool) -> Self {
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
            ui = ui.with_props(store);
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

fn identity(component: &str, object: &str) -> EditorObjectIdentity {
    EditorObjectIdentity {
        scene_instance_key: "video".into(),
        component_key: component.into(),
        object_key: object.into(),
        repeat_key: REPEAT_KEY.into(),
    }
}

#[test]
fn without_a_store_editable_components_draw_unchanged() {
    assert_eq!(
        drawn(&Probe::new(None, true), 20),
        drawn(&Probe::new(None, false), 20)
    );
}

#[test]
fn a_document_changes_what_editable_components_draw() {
    let store = Arc::new(PropsStore::default());
    store.listen();
    let probe = Probe::new(Some(store.clone()), true);
    let before = drawn(&probe, 20);
    assert!(before.contains("Ada"), "{before}");

    // the ids name each component for Studio's selection
    let ids: Vec<EditorObjectKey> = before
        .split(|c: char| !(c.is_ascii_alphanumeric() || c == '.'))
        .filter_map(EditorObjectKey::from_render_id)
        .collect();
    let keys: Vec<(String, String)> = ids
        .iter()
        .map(|key| (key.component_key.clone(), key.object_key.clone()))
        .collect();
    assert!(
        keys.contains(&("LowerThird".into(), "speaker".into())),
        "{before}"
    );
    assert!(keys.contains(&("Circle".into(), "dot".into())), "{before}");

    let speaker = identity("LowerThird", "speaker");
    let info = store.info(&speaker).expect("recorded while listening");
    assert_eq!(info.code_values["name"], "Ada");
    assert!(info.schema.fields.iter().any(|f| f.name == "role"));

    let mut doc = PropsDocument::default();
    doc.set(&speaker, "name", Some(json!("Grace")));
    doc.set(&identity("Circle", "dot"), "radius", Some(json!(90.0)));
    store.set(Some(doc));
    let after = drawn(&probe, 20);
    assert!(after.contains("Grace") && !after.contains("Ada"), "{after}");
    assert_ne!(before, after);
    let info = store.info(&speaker).unwrap();
    assert_eq!(info.code_values["name"], "Ada");
    assert_eq!(info.values["name"], "Grace");
    assert_eq!(
        store.info(&identity("Circle", "dot")).unwrap().values["radius"],
        90.0
    );

    store.set(None);
    assert_eq!(drawn(&probe, 20), before);
}
