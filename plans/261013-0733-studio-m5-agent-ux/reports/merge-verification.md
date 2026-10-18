# Merge verification

Worktree `fframes-m5`, branch `feat/studio-m5-ux`, HEAD `1659cbd` plus uncommitted test changes.

## Tests added (desktop/app/tests/agent_workflow)
- `sessions::a_reopened_session_restored_by_loading_shows_each_stored_row_once_while_the_agent_replays_it`
- `composer::opening_another_stored_session_clears_the_plan_and_commands_until_the_agent_publishes_again`
- `composer::starting_a_new_chat_clears_the_plan_and_commands_until_the_agent_publishes_again`

The fake agent already had the `replay-on-load` flag; no change to it. All three pass.

## Engine fix
None. The engine already clears commands and plan in `replace_conversation`.

## Existing test repaired (test-only)
`composer::a_loaded_session_does_not_replay_its_history_and_keeps_commands_and_plan` failed
deterministically with "timed out waiting for the load request". It enabled `load-session` after the
first task had stored its capabilities, so the stored capabilities said "no load" and the engine
rightly refused to resume. The test now enables `load-session` before the first task. Passes.

## Checks
- `cargo fmt --all -- --check`: pass (after one trailing blank line was removed by `cargo fmt`).
- `cargo clippy --workspace --all-targets -- -D warnings`: pass.
- `cargo test -p studio-engine -p studio-harness -p fframes-studio --no-fail-fast`: studio-engine and
  studio-harness all pass; fframes-studio had the one failure above; after the fix `-p fframes-studio` passes
  (agent_workflow 103 passed, 2 ignored).
- `cargo test -p studio-ui --lib`: 466 passed.
- `cargo test -p studio-ui --test parity --no-fail-fast`: run 1 hit the known
  `cancelling_an_export_leaves_no_file` hang (harness exit 101) and showed
  `an_unmigrated_project_exports_only_the_default_whatever_was_saved` as FAILED just before it (no detail
  was printed because the process was killed). Each passes alone (cancelling case 91 s). Full re-run: 66 passed.