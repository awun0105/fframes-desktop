# Xia: Zeron ACP agent auto-detection vs fframes desktop

Source: `C:\Users\Administrator\code\zeron` @ `344436efb28298a8b122ad1a970aff6be3880524` (crates/harness, crates/engine). License and attribution not yet reviewed (see unresolved questions). Mode: `--port` (default).

## Outcome

fframes Studio can already drive an ACP v1 agent and never stores credentials, so "no API key" already holds. What is missing is **auto-detection**: today every provider profile names a bare adapter executable (`claude-agent-acp`, `codex-acp`, `pi-acp`, `antigravity-acp`) that is searched only in `<app data>/adapters`. On this machine `claude.exe` exists in `~/.local/bin` but is not an ACP server, and nothing is found.

## How Zeron does it

| Aspect | Zeron | fframes today |
| --- | --- | --- |
| Agent catalog | Static `AcpAgentSpec` per agent: executable, args, env override (`GROK_EXECUTABLE`), pinned npm package, extra install dirs | Static `ProviderProfile` per agent: executable, args, auth env **names** |
| Search roots | `PATH` + login-shell `PATH` + per-agent dirs (`~/.local/bin`, `~/.npm-global/bin`, homebrew) + fnm/nvm/volta/pnpm/bun bins | Managed `<app data>/adapters` only; global `PATH` deliberately never used (`host.rs:546`) |
| Multiple copies | `newest_candidate` runs `--version` (2 s timeout, cached by path+mtime+size) and keeps the highest semver | First hit wins |
| Native ACP agents | devin `acp`, grok `agent stdio`, hermes `acp`, antigravity server | Only antigravity |
| Claude / Codex | Native drivers, not ACP | ACP adapters, which must be installed separately |
| Adapter install | Explicit user action; pinned npm package into a zeron-owned prefix, atomic rename, `node <entry>` launch with no `npx` | None; Studio does not install adapters |
| Auth | Agent's own login (`devin auth login`, ACP `authenticate`); no keys stored | Same: env var names only, plus the agent's own login |
| Detect vs install | Listing/detecting never runs an installer | "Check adapter" only starts the adapter in a scratch folder |

## Decision matrix

| Decision | Zeron's way | Our way | Recommendation |
| --- | --- | --- | --- |
| Search the global/login `PATH` | Yes | Refused by design (security: launch only what the user configured) | Search it only in a new read-only *detect* step; launching still needs the user to pick the result. Needs your approval |
| Claude and Codex | Native drivers | ACP adapter only | Detect the vendor CLI as a hint ("claude found, adapter missing") and offer an explicit adapter install; do not treat `claude.exe` as an ACP server |
| Agents with a native ACP mode | Per-agent args | Single exe + args per profile | Add gemini (`--experimental-acp`), opencode (`acp`), cursor (`agent acp`), grok, devin, hermes as profile entries; verify each flag against the installed CLI first |
| Version choice | Newest semver | First hit | Port `newest_candidate`, run through the existing process-tree manager |
| Probing | `--version` at detect time | Launch happens only at Check/Send | Detect by file existence only; run `--version` on demand |
| Auth | Agent login | Agent login | Keep; `auth_env_names` becomes optional and empty by default |

## Proposed change (if approved)

1. `studio-agent-spike/discovery.rs`: add `detect_agents(&DetectionSpec) -> Vec<Detected>` with search order managed dir, `PATH`, per-agent install dirs, Node version-manager bins. Windows must handle `.cmd` shims.
2. `provider-profiles.rs`: extend each profile with `cli_executable` and optional native ACP args; add the extra native-ACP profiles.
3. `conversation_panel` Setup tab: "Detect agents" button, a list showing found, version, signed-in-unknown, and one-click select. Opening a project still never launches anything.
4. Tests: fake executables in a temp dir covering order, multiple versions, `.cmd` on Windows, relative `PATH` entries rejected.
5. Docs: update `docs/desktop/README.md` (the Setup paragraph that says the global `PATH` is never consulted).

Rollback: the feature is additive, and removing the Detect action restores today's behavior.

## Unresolved questions

- May detection read the global `PATH`? This changes a documented security stance.
- Should Studio install the `claude-agent-acp` / `codex-acp` adapter on request, as Zeron's managed pinned npm prefix does?
- Zeron's license and `THIRD_PARTY_NOTICES.md` are not yet reviewed. Porting the design is fine, but copying code needs attribution.
