//! The component contract: [`Ui`], [`Component`], [`Themed`] and [`Children`].

use fframes::{
    BreakLinesOpts, FFramesContext, FontQuery, Frame, Scene, Svgr, TextOverflow,
    animation::{Animatable, KeyFramesAnimation},
    usvgr::svgtree::SvgAttributeValue,
};

use crate::{anim::Anim, explain::Space, theme::Theme};

/// What a component renders with: the local frame, the time left, the render context and
/// the theme.
///
/// A `Ui` lives on one render thread inside one `render_frame` call; build it at the top of
/// `render_frame` with [`Ui::new`], or at the top of a scene's with [`Ui::for_scene`].
pub struct Ui<'a, 'c, 'm> {
    /// Local time: shifted by every enclosing time container, so it starts at 0 where the
    /// component starts.
    pub frame: Frame,
    /// Frames from the local start to the end of the enclosing container, scene or video.
    /// `None` when a container runs until an end nobody gave.
    pub duration: Option<usize>,
    /// Media, fonts and scenes, the same context `render_frame` received.
    pub ctx: &'c FFramesContext<'a, 'm>,
    /// Defaults of every component.
    pub theme: &'a Theme,
    /// The coordinate system of the explain components, [`Space::FULL_HD`] unless a
    /// [`Canvas`](crate::explain::Canvas) sets another.
    pub space: Space,
}

/// A props struct that renders itself at the current time of a [`Ui`].
///
/// `render` consumes the props: owned strings move into the returned tree instead of being
/// cloned. A list of different components is a `Vec<BoxedComponent>`.
pub trait Component<'a> {
    fn render(self, ui: &mut Ui<'a, '_, '_>) -> Svgr<'a>;
}

/// The object-safe form of [`Component`], implemented for every component.
pub trait DynComponent<'a> {
    fn render_boxed(self: Box<Self>, ui: &mut Ui<'a, '_, '_>) -> Svgr<'a>;
}

impl<'a, T: Component<'a>> DynComponent<'a> for T {
    fn render_boxed(self: Box<Self>, ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
        (*self).render(ui)
    }
}

/// Any component behind a box, for lists of different components.
pub type BoxedComponent<'a> = Box<dyn DynComponent<'a> + 'a>;

impl<'a> Component<'a> for BoxedComponent<'a> {
    fn render(self, ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
        self.render_boxed(ui)
    }
}

impl<'a, T: Component<'a>> Component<'a> for Vec<T> {
    fn render(self, ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
        self.into_iter().map(|c| c.render(ui)).collect()
    }
}

impl<'a, T: Component<'a>> Component<'a> for Option<T> {
    fn render(self, ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
        self.map_or_else(Svgr::empty, |c| c.render(ui))
    }
}

/// Props whose defaults come from the theme, used through `..ui.defaults()`.
pub trait Themed {
    fn themed(theme: &Theme) -> Self;
}

/// Content of a container, rendered with the container's own `Ui` (shifted time, its
/// remaining duration). Build it with [`children`].
pub type Children<'a> = Box<dyn Fn(&mut Ui<'a, '_, '_>) -> Svgr<'a> + 'a>;

/// Boxes a closure as [`Children`].
pub fn children<'a>(f: impl Fn(&mut Ui<'a, '_, '_>) -> Svgr<'a> + 'a) -> Children<'a> {
    Box::new(f)
}

/// `value.into()` for a prop written as a tag attribute in `ui!`. A function, so a value
/// that already has the field's type is not reported as a useless conversion in the
/// caller's code.
#[doc(hidden)]
pub fn __prop<T: Into<U>, U>(value: T) -> U {
    value.into()
}

/// Children that draw nothing: the default of a container's `children`, so containers fill
/// their props with `..ui.defaults()` like other components.
pub(crate) fn no_children<'a>() -> Children<'a> {
    children(|_| Svgr::empty())
}

impl<'a, 'c, 'm> Ui<'a, 'c, 'm> {
    /// A `Ui` for a whole video: `duration` is the video's length in frames.
    pub fn new(frame: Frame, ctx: &'c FFramesContext<'a, 'm>, theme: &'a Theme) -> Self {
        Self {
            frame,
            duration: Some(ctx.duration_in_frames),
            ctx,
            theme,
            space: Space::FULL_HD,
        }
    }

    /// A `Ui` for `scene`: `duration` is the frames the scene renders, its own length plus
    /// its overlaps, so exit animations end with it. Scenes get the video's context, whose
    /// length is the whole video's.
    pub fn for_scene<S: Scene>(
        frame: Frame,
        ctx: &'c FFramesContext<'a, 'm>,
        theme: &'a Theme,
        scene: &S,
    ) -> Self {
        let duration = ctx
            .get_scene_info(scene)
            .map_or(ctx.duration_in_frames, |info| {
                info.duration_in_frames
                    .max(info.end_frame - info.start_frame)
            });
        Self {
            frame,
            duration: Some(duration),
            ctx,
            theme,
            space: Space::FULL_HD,
        }
    }

    /// A `Ui` whose time starts `offset` frames later and lasts `duration` frames. The caller
    /// checks `offset <= self.frame.index`.
    pub(crate) fn shifted(&self, offset: usize, duration: Option<usize>) -> Ui<'a, 'c, 'm> {
        debug_assert!(offset <= self.frame.index);
        Ui {
            frame: Frame::clone_with_scene_offset(&self.frame, offset),
            duration,
            ctx: self.ctx,
            theme: self.theme,
            space: self.space,
        }
    }

    /// A `Ui` at the same time whose explain components measure in `space`.
    pub fn with_space(&self, space: Space) -> Ui<'a, 'c, 'm> {
        Ui {
            frame: self.frame.clone(),
            duration: self.duration,
            ctx: self.ctx,
            theme: self.theme,
            space,
        }
    }

    /// Renders a component at the current time.
    pub fn show(&mut self, component: impl Component<'a>) -> Svgr<'a> {
        component.render(self)
    }

    /// The theme's defaults for `T`, for struct update syntax: `FadeIn { delay: 0.2,
    /// ..ui.defaults() }`.
    pub fn defaults<T: Themed>(&self) -> T {
        T::themed(self.theme)
    }

    /// Local time in seconds.
    pub fn seconds(&self) -> f32 {
        self.frame.seconds()
    }

    /// `duration` in seconds.
    pub fn duration_seconds(&self) -> Option<f32> {
        self.duration.map(|d| self.frame.frame_to_second(d))
    }

    /// Converts seconds to frames at the video's frame rate.
    pub fn frames(&self, seconds: f32) -> usize {
        self.frame.second_to_frame(seconds.max(0.0))
    }

    /// The value of an animated prop at the current time.
    pub fn sample<T>(&self, anim: &Anim<'a, T>) -> T
    where
        T: Animatable + Copy + Default + std::fmt::Debug,
    {
        match anim {
            Anim::Fixed(value) => *value,
            Anim::Keyframes(keyframes) => self.frame.animate(keyframes),
        }
    }

    /// Samples a stored `timeline!` at the current time.
    pub fn animate<T>(&self, keyframes: &KeyFramesAnimation<T>) -> T
    where
        T: Animatable + Copy + Default + std::fmt::Debug,
    {
        self.frame.animate(keyframes)
    }

    /// Width in pixels of text borrowed for the video's lifetime, see
    /// [`Frame::text_width`]. `None` when the font cannot be resolved.
    pub fn text_width(&mut self, font: FontQuery, text: &'a str) -> Option<usize> {
        self.frame.text_width(self.ctx, font, text)
    }

    /// Shortens text to `max_width` pixels, see [`Frame::text_fit`].
    pub fn text_fit(
        &mut self,
        font: FontQuery,
        text: &'a str,
        max_width: usize,
        overflow: TextOverflow,
    ) -> Option<std::borrow::Cow<'a, str>> {
        self.frame
            .text_fit(self.ctx, font, text, max_width, overflow)
    }

    /// A multi-line `<text>` element, see [`Frame::text_break_lines`].
    pub fn text_break_lines<X, Y>(
        &mut self,
        text: &str,
        opts: BreakLinesOpts<'a, X, Y>,
    ) -> Option<Svgr<'a>>
    where
        X: Into<SvgAttributeValue<'a>> + std::hash::Hash + Default + Copy,
        Y: Into<SvgAttributeValue<'a>> + std::hash::Hash + Default + Copy,
    {
        self.frame.text_break_lines(self.ctx, text, opts)
    }

    /// Width in pixels of a string of any lifetime, such as one built this frame. It sums the
    /// advance of every character like [`Frame::text_width`] does, so both give the same
    /// number; only `text_width` caches it. `None` when the font cannot be resolved.
    pub fn measure(&self, font: FontQuery, text: &str) -> Option<f32> {
        let face = self.ctx.font_source?.resolve_font(
            font.family,
            font.weight,
            font.style,
            font.stretch,
        )?;
        if matches!(text, "" | "\n" | "\r") {
            return Some(0.0);
        }
        let width: usize = if face.is_monospaced()? {
            let advance = face.resolve_char_width(font.size, 'm')?;
            advance * text.chars().count()
        } else {
            text.chars()
                .map(|c| face.resolve_char_width(font.size, c).unwrap_or_default())
                .sum()
        };
        Some(width as f32)
    }

    /// A deterministic SVG id for a clip path, gradient or filter of kind `kind`: built from
    /// `key` when given, otherwise from the item `index`. Two components of one kind in one
    /// frame need different keys or indexes.
    pub fn id(&self, key: Option<&str>, index: usize, kind: &str) -> String {
        id(key, index, kind)
    }
}

fn id(key: Option<&str>, index: usize, kind: &str) -> String {
    let clean = |s: &str| -> String {
        s.chars()
            .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
            .collect()
    };
    match key {
        Some(key) => format!("kit-{}-k-{}", clean(kind), clean(key)),
        None => format!("kit-{}-i{index}", clean(kind)),
    }
}

#[cfg(test)]
mod tests {
    use super::id;

    #[test]
    fn ids_are_deterministic_and_distinct() {
        assert_eq!(id(None, 0, "clip"), id(None, 0, "clip"));
        assert_ne!(id(None, 0, "clip"), id(None, 1, "clip"));
        assert_ne!(id(Some("a"), 0, "clip"), id(Some("b"), 0, "clip"));
        assert_ne!(id(None, 0, "clip"), id(None, 0, "gradient"));
        // a key never collides with an index
        assert_ne!(id(Some("i0"), 0, "clip"), id(None, 0, "clip"));
        assert_eq!(id(Some("my card!"), 0, "clip"), "kit-clip-k-my-card-");
    }
}
