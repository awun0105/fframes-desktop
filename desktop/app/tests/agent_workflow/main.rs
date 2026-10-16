//! The native agent workflow against real subprocesses: the scripted ACP peer
//! (`tests/support/acp-agent.py`), the deterministic fake preview worker and the engine's
//! real controller. The peer proves transport and workflow only; it never qualifies a
//! provider, an adapter or authentication.
#![allow(unused_imports)]

use fframes_studio_protocol::{EditorFrameStatus, PreviewIdentity};

// The shared harness (World, Scripted compiler, script helpers) lives in one file so
// the scoped-editing suite runs against exactly the same subprocess setup.
include!("../../../crates/studio-engine/tests/support/workflow_world.rs");

struct Sink {
    coordinator: Mutex<Option<Arc<PreviewCoordinator>>>,
    fail: AtomicBool,
    adopted: Mutex<Vec<String>>,
}

impl PreviewHandoff for Sink {
    fn handoff(&self, handoff: PromotionHandoff) -> Result<(), String> {
        if self.fail.load(Ordering::SeqCst) {
            return Err("preview worker refused".into());
        }
        let staged = handoff.staged.ok_or("no staged preview")?;
        let authorization = handoff
            .promotion
            .authorization
            .as_ref()
            .ok_or("the source changed again")?;
        let identity = staged.ready().identity().clone();
        let coordinator = self
            .coordinator
            .lock()
            .clone()
            .ok_or("the coordinator is gone")?;
        coordinator
            .adopt(
                staged,
                authorization,
                SeekIntent {
                    identity,
                    serial: 0,
                    position: 5,
                    scale: 1.,
                },
            )
            .map_err(|e| e.to_string())?;
        self.adopted
            .lock()
            .push(handoff.promotion.record.published.as_str().to_owned());
        Ok(())
    }
}

fn wait_ready(p: &PreviewCoordinator) -> Arc<studio_engine::ReadyPreview> {
    let deadline = Instant::now() + WAIT;
    loop {
        let e = p.events();
        if let Some((_, error)) = e.error {
            panic!("preview failed: {error}");
        }
        if let Some(ready) = e.ready {
            return ready;
        }
        assert!(Instant::now() < deadline, "no ready preview");
        std::thread::sleep(Duration::from_millis(10));
    }
}

impl World {
    fn wait_phase_snapshot(&self, phase: TaskPhase) -> Arc<WorkflowSnapshot> {
        self.wait(&format!("phase {phase:?}"), |s| {
            s.task.as_ref().is_some_and(|t| t.phase == phase)
        })
    }
}

mod claude_wire;
mod close;
mod codex_wire;
mod composer;
mod cycles;
mod failures;
mod happy_path;
mod manual_review;
mod permissions;
mod preview_handoff;
mod queueing;
mod readiness;
mod real_sdk;
mod recovery;
mod regressions;
mod repair;
mod resource_bounds;
mod sessions;
mod stop;
mod undo;
