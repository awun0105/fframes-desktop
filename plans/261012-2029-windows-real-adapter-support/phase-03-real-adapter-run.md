# Phase 3: Real Claude adapter run

## Needs the user

- Claude ACP adapter selected in Setup and signed in (Check adapter shows Ready).
- Consent to real edits in a throwaway scratch folder using their sign-in.

## Steps

1. `cargo run --release -p fframes-studio -- qualify-writer` (selected profile).
2. Restart the app or press Check adapter; Setup must show "qualified for this exact
   adapter and platform".
3. Run a small real task on a test project: it must reach review (Apply still blocked
   until Phase 4) and the candidate must export.

## Notes

The qualification is bound to the exact executable bytes and arguments: an adapter
update needs a new run.
