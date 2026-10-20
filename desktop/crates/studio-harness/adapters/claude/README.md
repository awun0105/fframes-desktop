# Pinned Claude ACP advisor correction

Studio installs the locked `claude-agent-acp` 0.89.1 package plus `advisor-result.js`.
That upstream build drops `advisor_tool_result` while retaining the corresponding
server call in its unfinished set. A successful edit can therefore end with an
`incomplete_tool_call` error despite the API having returned the advisor result.

The correction closes only the call whose actual result arrived. Advisor errors
remain failed tool calls, duplicate results emit nothing, and unrelated unfinished
calls stay unfinished. The driver's incomplete-turn capture gate stays in effect.

`src/adapter_install/claude_patch.rs` checks SHA-256 before and after the change.
The corrected artifact has its own `advisor-result-v1` installation directory;
existing uncorrected installations are left alone and are not reused as this
revision. For an existing ACP profile, use **Install adapter** and **Select** to use
the corrected install. Saved custom launch paths are not rewritten.

Run the JavaScript conversion regression against the installed module; no model call is
needed:

```text
node advisor-result.test.mjs <installed-package>/dist/acp-agent.js
```

The conversion regression fails on the unmodified 0.89.1 module. Remove this patch
when an upstream version handles the result, and update both the lock and tests.
Upstream source: https://github.com/agentclientprotocol/claude-agent-acp/blob/v0.89.1/src/acp-agent.ts
