pub mod app;
pub mod assets;
pub use studio_ui::panels::conversation as conversation_panel;
pub use studio_ui::shell as studio_shell;
pub use studio_ui::{actions, canvas_view, frame_image, preset_panel, setup_view, timeline_view};
pub mod selection_spike;

pub use app::StudioSpikeApp;
pub use frame_image::{
    ConversionError, ImagePresentationManager, convert_rgba_to_gpui_bgra, create_render_image,
};
pub use selection_spike::{SelectedElementInfo, SelectionError, SelectionSpike};
pub use setup_view::{SetupState, SetupView};

pub mod presentation_stress;
pub mod preview_element;
pub mod ui;

pub mod agent_spike;
pub mod stress_metrics;
