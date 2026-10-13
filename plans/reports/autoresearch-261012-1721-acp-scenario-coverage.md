# ACP scenario coverage (zeron comparison)

Outcome: `studio-agent-spike` gained 11 real-subprocess ACP driver tests (acp_v1: 28 → 41) and fixed one driver bug they exposed. The full crate suite (54 + 41 + 15) passed three runs in a row, and `cargo clippy -p studio-agent-spike --tests -- -D warnings` and `cargo fmt` are clean.

Reference: `~/code/zeron/crates/harness/tests/acp.rs` and `crates/engine/tests/acp_lifecycle.rs`. Only the generic ACP transport cases were ported. Provider-specific cases (antigravity/devin sign-in and models, grok subagent tailing, steering extensions) were skipped on purpose.

Contract: the tests pin the existing fframes driver contract. Nothing was changed to make the driver behave like zeron; the single driver edit is the bug fix described under Findings.

## New scenarios (tests/acp_v1.rs, peer modes in tests/acp-peer.py)

| Scenario | Peer mode |
|---|---|
| Image prompts: no negotiated capability, >3 images, non-PNG, empty, >4 MiB, oversized text; rejections never start a turn | `image-ok` |
| Thought and agent text keep roles and order; non-text content becomes a placeholder | `thoughts` |
| plan / available_commands / session_info updates emit nothing and are not "late" | `inert-updates` |
| Adapter-pushed `current_mode_update` / `config_option_update` update `Options` | `push-options` |
| Turn with no output completes exactly once | `empty-turn` |
| Error response to `session/prompt` is `PromptRejected` | `prompt-error` |
| Back-to-back duplicate prompt response is ignored (`dup-finish`); a stale response arriving during turn 2 never settles it | `dup-finish`, `stale-finish` |
| Reply ending in a secret prefix is released with its own turn | `secret-tail` |
| Adapter dies while idle: one failure, new prompts refused | `idle-crash` |
| Permission request for a foreign session is a protocol violation | `foreign-perm` |
| Unknown adapter request gets an error reply and the turn continues | `unknown-method` |
| Blank lines and CRLF framing | `crlf` |

## Findings

- **Fixed: held secret prefix leaked into the next turn.** At the end of a turn the stream redactor kept trailing text that was a prefix of a configured secret. A reply ending in such a character (for example "s" for a secret starting with "s") lost it, and it reappeared at the start of the next turn ("tails" then "Hello" produced "tail" then "sHello"). The turn-end and restore-end flushes in `src/driver/runtime.rs` now release held text with its own turn (`finish_streams_at_turn_end`). Mid-turn event boundaries still hold the prefix. Pinned by `a_reply_ending_in_a_secret_prefix_is_released_with_its_own_turn`.
- **Transcript merging (not changed).** Consecutive agent messages from different turns coalesce into one transcript entry ("tailsHello").
- **Strict framing.** A non-JSON line on stdout fails the driver (`malformed-mid`), whereas zeron tolerates stdout noise. Blank lines and CRLF are accepted.
- **Unmapped updates.** Plans, command lists, diffs, tool output and usage are not mapped to events. This is the documented contract. Adding them would change `AgentEventKind`, which `desktop/app` consumes.

## Open questions

- Should the driver surface plans, available commands, tool output/diffs and context usage as events, as zeron does? It changes a public contract and needs your decision.

## Not covered here

POSIX-only helper-process tests stay `cfg(unix)`. Real-adapter lifecycle runs (zeron's `real_acp_lifecycle`) need installed adapters and credentials.

## Files changed

`tests/acp_v1.rs`, `tests/acp-peer.py`, and `src/driver/runtime.rs` (the redactor flush fix only). The other modified files under `src/` predate this work.
