//! The renderer every worker of this Studio session is asked for with `--renderer`.
//!
//! It is the Settings choice (`auto` until one is made), unless Studio was started with
//! `FFRAMES_STUDIO_RENDERER` for testing (read once, never passed to a child). After a
//! worker that drew on the GPU stops responding, the rest of the session uses the
//! built-in CPU renderer.
use parking_lot::Mutex;
use std::sync::OnceLock;

/// What a worker is asked to render with; see `fframes_studio_runtime::RendererPreference`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RendererChoice {
    /// Skia on the GPU when one works, the built-in CPU renderer otherwise.
    #[default]
    Auto,
    /// Skia on the GPU, or no preview.
    Gpu,
    /// Skia on the CPU: draws shaders, slower.
    SkiaCpu,
    /// The built-in CPU renderer: no shaders.
    Cpu,
}

impl RendererChoice {
    pub const ALL: [Self; 4] = [Self::Auto, Self::Gpu, Self::SkiaCpu, Self::Cpu];

    /// The worker argument value.
    pub fn as_arg(self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::Gpu => "gpu",
            Self::SkiaCpu => "skia-cpu",
            Self::Cpu => "cpu",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|choice| choice.as_arg() == value.trim())
    }
}

#[derive(Debug, Default)]
struct Session {
    choice: RendererChoice,
    /// Why the session left the GPU, once it did.
    demoted: Option<String>,
}

fn session() -> &'static Mutex<Session> {
    static SESSION: OnceLock<Mutex<Session>> = OnceLock::new();
    SESSION.get_or_init(Mutex::default)
}

fn environment_override() -> Option<RendererChoice> {
    static OVERRIDE: OnceLock<Option<RendererChoice>> = OnceLock::new();
    *OVERRIDE.get_or_init(|| {
        std::env::var("FFRAMES_STUDIO_RENDERER")
            .ok()
            .and_then(|value| RendererChoice::parse(&value))
    })
}

/// The renderer the next worker is launched with.
pub fn current() -> RendererChoice {
    let session = session().lock();
    if session.demoted.is_some() {
        return RendererChoice::Cpu;
    }
    environment_override().unwrap_or(session.choice)
}

/// Applies the Settings choice to workers launched from now on.
pub fn set_choice(choice: RendererChoice) {
    session().lock().choice = choice;
}

/// After a worker that rendered on the GPU (`backend`) hung or exited during a frame, the
/// rest of the session renders with the built-in CPU renderer. Returns `true` the first
/// time; other backends never demote the session.
pub fn gpu_worker_failed(backend: &str, reason: &str) -> bool {
    if backend != "skia-gpu" {
        return false;
    }
    let mut session = session().lock();
    if session.demoted.is_some() {
        return false;
    }
    session.demoted = Some(reason.to_owned());
    true
}

/// Why this session renders on the CPU after a GPU failure.
pub fn demotion() -> Option<String> {
    session().lock().demoted.clone()
}

/// The `--renderer` arguments of a worker launched now.
pub fn worker_args() -> [&'static str; 2] {
    ["--renderer", current().as_arg()]
}

#[cfg(test)]
pub(crate) fn reset_for_tests() {
    *session().lock() = Session::default();
}

/// Serializes the tests of this process that change the session.
#[cfg(test)]
pub(crate) fn tests_lock() -> parking_lot::MutexGuard<'static, ()> {
    static LOCK: Mutex<()> = Mutex::new(());
    LOCK.lock()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn choices_round_trip_through_their_worker_argument() {
        for choice in RendererChoice::ALL {
            assert_eq!(RendererChoice::parse(choice.as_arg()), Some(choice));
        }
        assert_eq!(RendererChoice::parse("vulkan"), None);
    }

    #[test]
    fn only_a_gpu_worker_failure_moves_the_session_to_the_cpu_renderer_once() {
        let _serial = crate::services::renderer_preference::tests_lock();
        reset_for_tests();
        set_choice(RendererChoice::SkiaCpu);
        assert!(!gpu_worker_failed("skia-cpu", "slow frame"));
        assert!(!gpu_worker_failed("cpu", "slow frame"));
        assert_eq!(
            current(),
            environment_override().unwrap_or(RendererChoice::SkiaCpu)
        );
        assert!(gpu_worker_failed("skia-gpu", "frame deadline"));
        assert!(!gpu_worker_failed("skia-gpu", "again"), "reported once");
        assert_eq!(current(), RendererChoice::Cpu);
        assert_eq!(demotion().as_deref(), Some("frame deadline"));
        set_choice(RendererChoice::Gpu);
        assert_eq!(
            current(),
            RendererChoice::Cpu,
            "the session stays off the GPU"
        );
        reset_for_tests();
    }
}
