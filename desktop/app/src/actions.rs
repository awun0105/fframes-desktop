//! The studio's commands as gpui actions. The shell handles every one at its root, so an
//! action dispatched anywhere in the window (a key binding, a menu, a host driving the
//! app) reaches the same code as the button that offers it.
use gpui::actions;

actions!(
    studio,
    [
        /// Choose a project folder and open it.
        OpenProject,
        /// Build the open project's preview.
        BuildPreview,
        /// Stop the preview build in flight.
        CancelPreview,
        /// Choose a destination and export the accepted revision as MP4.
        ExportVideo,
        /// Stop the export in flight.
        CancelExport,
    ]
);
