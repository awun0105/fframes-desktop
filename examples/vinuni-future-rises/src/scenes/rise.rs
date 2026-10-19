//! Scene 19, 146–156 s: the tower at night under the title, which settles under a white
//! band opening with the official `VinUniversity` logo over the aurora, pushing in slowly;
//! the credits, then black.

use fframes::{Duration, FFramesContext, Frame, Scene, ShaderUniforms, Svgr};

use super::Sfx;
use crate::{
    AMBER, AURORA, DISPLAY, Grade, LABEL, MUTED, SANS, ease_in_out, ease_out, footage, progress,
    punch,
};

super::scene_length!(RiseScene, 18);

const TITLE_AT: f32 = 0.3;
const OPEN_AT: f32 = 2.3;
const BAND_W: f32 = 1300.0;
const BAND_H: f32 = 300.0;

pub(crate) fn sfx() -> Vec<Sfx> {
    vec![
        ("piano.wav", TITLE_AT, -8.0, 0.0),
        ("sub.wav", OPEN_AT, -6.0, 0.0),
        ("chime.wav", OPEN_AT, -10.0, 0.0),
    ]
}

impl Scene for RiseScene {
    fn duration(&self) -> Duration<'_> {
        Duration::Seconds(Self::LENGTH)
    }

    fn editor_instance_key(&self) -> Option<&str> {
        Some("rise")
    }

    fn render_frame<'a>(&'a self, frame: Frame, ctx: &FFramesContext<'a, '_>) -> Svgr<'a> {
        let t = frame.seconds();
        let tower = footage(
            &frame,
            ctx,
            "tower.mp4",
            0.0,
            Grade {
                amount: 0.25,
                zoom: 1.0 + 0.03 * t,
                exposure: 0.9,
                ..Default::default()
            },
            (0.0, 0.0, 1920.0, 1080.0),
        );
        let aurora = AURORA.draw(
            &frame,
            ShaderUniforms::new().float("uAlpha", ease_out(progress(t, 1.6, 0.8))),
        );
        let open = punch(t, OPEN_AT).max(0.0);
        let h = (BAND_H * open).max(3.0);
        let band_y = 540.0 - h / 2.0;
        let logo = match ctx.get_image("logo.png") {
            Some(image) => {
                fframes::svgr!(<image href={image.href()} x="510" y="454" width="900" height="172" />)
            }
            None => Svgr::empty(),
        };
        // the title rises in over the tower, then settles under the band
        let title = ease_out(progress(t, TITLE_AT, 0.8));
        let settle = ease_in_out(progress(t, OPEN_AT - 0.2, 0.7));
        let title_y = 900.0 - 110.0 * settle + (1.0 - title) * 30.0;
        let title_scale = 1.0 - 0.3 * settle;
        let band = progress(t, OPEN_AT - 0.15, 0.15);
        let url = ease_out(progress(t, OPEN_AT + 0.9, 0.5));
        let out = progress(t, Self::LENGTH - 0.6, 0.55);
        let push = 1.0 + 0.04 * ease_in_out(progress(t, OPEN_AT, Self::LENGTH - OPEN_AT));
        fframes::svgr!(<g>
            <image href={aurora.href()} x="0" y="0" width="1920" height="1080" />
            <g opacity={1.0 - progress(t, 1.4, 0.5)}>{tower}</g>
            <defs>
                <clipPath id="rise-band"><rect x={960.0 - BAND_W / 2.0} y={band_y} width={BAND_W} height={h} /></clipPath>
            </defs>
            <g transform={format!("translate(960 540) scale({push}) translate(-960 -540)")}>
            <g opacity={band}>
                <rect x={960.0 - BAND_W / 2.0} y={band_y} width={BAND_W} height={h} fill="#ffffff" />
                <g clip-path="url(#rise-band)" opacity={progress(t, OPEN_AT + 0.1, 0.3)}>{logo}</g>
                <rect x={960.0 - BAND_W / 2.0} y={band_y - 3.0} width={BAND_W} height="3" fill={AMBER} />
                <rect x={960.0 - BAND_W / 2.0} y={band_y + h} width={BAND_W} height="3" fill={AMBER} />
            </g>
            <g opacity={title} transform={format!("translate(960 {title_y}) scale({title_scale}) translate(-960 0)")}>
                <text x="960" y="0" text-anchor="middle" font-family={DISPLAY} font-weight="700" font-size="64" letter-spacing="10" fill="#ffffff">"WHERE THE FUTURE RISES"</text>
            </g>
            </g>
            <text x="960" y="870" text-anchor="middle" font-family={DISPLAY} font-weight={LABEL} font-size="30" letter-spacing="3" fill="#ffffff" opacity={url}>"vinuni.edu.vn"</text>
            <text x="960" y="1012" text-anchor="middle" font-family={SANS} font-size="15" letter-spacing="2" fill={MUTED} opacity={0.85 * url}>"Footage, photos, VinUni & college logos © VinUniversity (vinuni.edu.vn, its college sites, official YouTube channel) · Made with fframes"</text>
            <text x="960" y="1040" text-anchor="middle" font-family={SANS} font-size="15" letter-spacing="2" fill={MUTED} opacity={0.85 * url}>"University and company logos are trademarks of their owners, shown to name partners and employers"</text>
            <rect width="1920" height="1080" fill="#000000" opacity={out} />
        </g>)
    }
}
