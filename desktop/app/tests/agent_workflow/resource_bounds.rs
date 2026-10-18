//! The resident transcript and streaming text stay bounded.

use super::*;

#[test]
fn the_resident_transcript_is_bounded_and_older_rows_are_paged_from_the_log() {
    use studio_engine::agent::log::RowLimits;
    let w = World::with(Options {
        row_limits: Some(RowLimits {
            max_rows: 20,
            ..RowLimits::default()
        }),
        ..Options::default()
    });
    let mut script = good("flooded");
    script["flood_tools"] = json!(150);
    w.submit(&script);
    let task = w.wait_task(None);
    assert_eq!(task.phase, TaskPhase::Accepted, "{:?}", task.error);
    let s = w.snap();
    assert!(
        s.rows.len() <= 20 && s.resources.resident_rows <= 20,
        "{}",
        s.rows.len()
    );
    assert!(s.older_rows);
    assert!(s.resources.resident_bytes <= s.resources.max_resident_bytes);
    let first = s.rows[0].id;
    let page = w.wf().history_page(first, 1000).unwrap();
    assert_eq!(page.len(), MAX_HISTORY_PAGE, "pages are clamped");
    assert!(page.iter().all(|r| r.id < first));
    assert!(page.windows(2).all(|p| p[0].id < p[1].id));
    assert!(page.iter().any(|r| matches!(&r.kind, RowKind::Tool(_))));
    // Paging back reaches the persisted brief, the oldest row of all.
    let mut before = page[0].id;
    let mut oldest = page;
    loop {
        let older = w.wf().history_page(before, MAX_HISTORY_PAGE).unwrap();
        if older.is_empty() {
            break;
        }
        before = older[0].id;
        oldest = older;
    }
    assert!(matches!(
        &oldest[0].kind,
        RowKind::User {
            source: UserSource::Brief,
            ..
        }
    ));
}

#[test]
fn streaming_text_is_bounded_by_bytes_and_never_one_unbounded_row() {
    use studio_engine::agent::log::RowLimits;
    let w = World::with(Options {
        row_limits: Some(RowLimits {
            max_bytes: 48 * 1024,
            ..RowLimits::default()
        }),
        ..Options::default()
    });
    let mut script = good("long");
    script["flood"] = json!(20000);
    w.submit(&script);
    let task = w.wait_task(None);
    assert_eq!(task.phase, TaskPhase::Accepted, "{:?}", task.error);
    let s = w.snap();
    let biggest = s.rows.iter().map(|r| r.estimated_bytes()).max().unwrap();
    assert!(
        biggest <= MAX_ROW_TEXT_BYTES + ROW_OVERHEAD_BYTES + 64,
        "{biggest}"
    );
    // The newest row is never evicted; the rest fits the budget (plus that one row).
    assert!(s.resources.resident_bytes <= 48 * 1024 + MAX_ROW_TEXT_BYTES + ROW_OVERHEAD_BYTES);
    assert!(s.older_rows);
}

/// The published count of owned processes follows a process that starts and ends without
/// an event of the workflow's own (as a preview build or an asynchronous teardown does),
/// with no refresh asked for.
#[test]
fn the_owned_process_count_follows_processes_that_end_between_events() {
    let w = World::new();
    w.wait("the opened workflow", |s| s.resources.owned_processes == 0);
    let mut sleeper = studio_bootstrap::process::SpawnOptions::new(python());
    sleeper.args(["-c", "import time; time.sleep(60)"]);
    let child = w.ctl().lock().processes.spawn(sleeper).unwrap();
    w.wait("the process counted", |s| s.resources.owned_processes == 1);
    child.lock().kill_forcefully().unwrap();
    w.wait("the ended process no longer counted", |s| {
        s.resources.owned_processes == 0
    });
    assert_eq!(w.processes(), 0);
}
