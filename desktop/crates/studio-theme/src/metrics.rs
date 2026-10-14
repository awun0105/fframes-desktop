//! Spacing, radii and sizes shared by every Studio surface.

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Metrics {
    pub space_1: f32,
    pub space_2: f32,
    pub space_3: f32,
    pub space_4: f32,
    pub space_6: f32,
    pub radius_control: f32,
    pub radius_item: f32,
    pub radius_segment: f32,
    pub radius_card: f32,
    pub radius_popover: f32,
    pub radius_tab: f32,
    pub radius_frame: f32,
    pub control_h: f32,
    pub select_h: f32,
    pub input_h: f32,
    pub field_row_h: f32,
    pub icon_sm: f32,
    pub icon_md: f32,
    pub header_h: f32,
    pub tab_bar_h: f32,
    pub toolbar_h: f32,
    pub transport_h: f32,
    pub gutter: f32,
    pub left_column_w: f32,
    pub agent_panel_w: f32,
    pub timeline_default_h: f32,
    pub timeline_min_h: f32,
    pub player_min_h: f32,
    pub ruler_h: f32,
    pub track_h: f32,
    pub track_gap: f32,
    pub track_gutter_w: f32,
    /// Button padding.
    pub control_pad_x: f32,
    pub control_pad_y: f32,
}

pub const METRICS: Metrics = Metrics {
    space_1: 4.,
    space_2: 8.,
    space_3: 12.,
    space_4: 16.,
    space_6: 24.,
    radius_control: 7.,
    radius_item: 6.,
    radius_segment: 8.,
    radius_card: 12.,
    radius_popover: 12.,
    radius_tab: 9.,
    radius_frame: 6.,
    control_h: 28.,
    select_h: 30.,
    input_h: 32.,
    field_row_h: 34.,
    icon_sm: 14.,
    icon_md: 16.,
    header_h: 52.,
    tab_bar_h: 46.,
    toolbar_h: 44.,
    transport_h: 48.,
    gutter: 8.,
    // fframes value: today's left column width (Cap has no left column).
    left_column_w: 248.,
    agent_panel_w: 400.,
    timeline_default_h: 260.,
    timeline_min_h: 240.,
    player_min_h: 320.,
    ruler_h: 26.,
    track_h: 44.,
    track_gap: 6.,
    track_gutter_w: 104.,
    control_pad_x: 12.,
    control_pad_y: 7.,
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn metrics_match_cap_spec() {
        let m = METRICS;
        assert_eq!(
            [m.space_1, m.space_2, m.space_3, m.space_4, m.space_6],
            [4., 8., 12., 16., 24.]
        );
        assert_eq!(
            [
                m.radius_control,
                m.radius_item,
                m.radius_segment,
                m.radius_card,
                m.radius_popover,
                m.radius_tab,
                m.radius_frame
            ],
            [7., 6., 8., 12., 12., 9., 6.]
        );
        assert_eq!(
            [
                m.control_h,
                m.select_h,
                m.input_h,
                m.field_row_h,
                m.icon_sm,
                m.icon_md
            ],
            [28., 30., 32., 34., 14., 16.]
        );
        assert_eq!(
            [
                m.header_h,
                m.tab_bar_h,
                m.toolbar_h,
                m.transport_h,
                m.gutter,
                m.left_column_w,
                m.agent_panel_w,
                m.timeline_default_h,
                m.timeline_min_h,
                m.player_min_h
            ],
            [52., 46., 44., 48., 8., 248., 400., 260., 240., 320.]
        );
        assert_eq!(
            [m.ruler_h, m.track_h, m.track_gap, m.track_gutter_w],
            [26., 44., 6., 104.]
        );
    }
}
