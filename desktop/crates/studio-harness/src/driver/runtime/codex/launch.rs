//! Codex launch: user flags, the version gate and the thread and turn settings that keep
//! every approval with the user and every write inside the draft.

use super::super::{
    McpStdioServer,
    native_launch::{self, UserArgs, Version},
};
use serde_json::{Map, Value, json};
use std::path::Path;

pub(crate) const PRODUCT: &str = "Codex";
/// The oldest Codex release the app-server wire was validated against (S5 and the
/// generated schema).
pub(crate) const MIN_VERSION: Version = (0, 162, 1);
/// Asks before every command and file change; `on-request` asked for nothing in S5.
pub(crate) const APPROVAL_POLICY: &str = "untrusted";

/// User arguments a Codex adapter may carry: the model (`thread/start`) and the reasoning
/// effort (`turn/start`). Policies, sandbox, config and MCP servers are the driver's own.
const USER_FLAGS: [&str; 2] = ["--model", "--effort"];

pub(crate) fn split_user_args(args: &[String]) -> Result<UserArgs, String> {
    native_launch::split_user_args(PRODUCT, &USER_FLAGS, args)
}

/// `codex --version`, cached, checked against [`MIN_VERSION`].
pub(crate) fn version_gate(
    executable: &Path,
    prefix: &[String],
    env: &studio_bootstrap::ChildEnvironment,
) -> Result<Version, String> {
    native_launch::version_gate(PRODUCT, MIN_VERSION, executable, prefix, env)
}

pub(crate) fn check_version(version: Version) -> Result<(), String> {
    native_launch::check_version(PRODUCT, MIN_VERSION, version)
}

/// The full argument list after the program prefix: no `-c` overrides, the session's
/// settings travel in `thread/start` so no secret reaches the command line.
pub(crate) fn wire_args(user: &UserArgs) -> Vec<String> {
    let mut args = user.prefix.clone();
    args.push("app-server".into());
    args
}

/// `thread/start` / `thread/resume` config overrides: the user's own MCP servers and the
/// ChatGPT apps connector are switched off and only the studio's servers run.
/// `-c mcp_servers={}` and a whole `mcp_servers` object both leave the user's servers on
/// (they merge), so each one is disabled by name.
pub(crate) fn thread_config(user_servers: &[String], servers: &[McpStdioServer]) -> Value {
    let mut config = Map::new();
    config.insert("features.apps".into(), Value::Bool(false));
    for name in user_servers {
        if servers.iter().any(|server| &server.name == name) {
            continue;
        }
        config.insert(format!("mcp_servers.{name}.enabled"), Value::Bool(false));
    }
    for server in servers {
        let key = |field: &str| format!("mcp_servers.{}.{field}", server.name);
        config.insert(key("command"), json!(server.command));
        config.insert(key("args"), json!(server.args));
        config.insert(
            key("env"),
            Value::Object(
                server
                    .env
                    .iter()
                    .map(|(name, value)| (name.clone(), Value::from(value.clone())))
                    .collect(),
            ),
        );
        config.insert(key("enabled"), Value::Bool(true));
    }
    Value::Object(config)
}

/// The turn sandbox: writes only inside the draft (the thread's working directory), no
/// network, and neither `/tmp` nor `$TMPDIR` as extra writable roots.
pub(crate) fn sandbox_policy() -> Value {
    json!({
        "type": "workspaceWrite",
        "writableRoots": [],
        "networkAccess": false,
        "excludeTmpdirEnvVar": true,
        "excludeSlashTmp": true,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn strings(args: &[&str]) -> Vec<String> {
        args.iter().map(|s| (*s).to_owned()).collect()
    }

    #[test]
    fn only_model_and_effort_are_accepted_and_nothing_else_reaches_the_command_line() {
        let user = split_user_args(&strings(&["--model", "m", "--effort", "low"])).unwrap();
        assert_eq!(wire_args(&user), strings(&["app-server"]));
        for refused in [
            &["-c", "approval_policy=never"][..],
            &["--dangerously-bypass-approvals-and-sandbox"],
            &["--sandbox", "danger-full-access"],
            &["--config", "x"],
        ] {
            assert!(split_user_args(&strings(refused)).is_err(), "{refused:?}");
        }
    }

    #[test]
    fn user_servers_are_disabled_and_only_studio_servers_run() {
        let server = McpStdioServer::new(
            "studio",
            std::env::current_exe().unwrap(),
            vec!["--serve".into()],
            vec![("TOKEN".into(), "v".into())],
        )
        .unwrap();
        let config = thread_config(&strings(&["rea", "studio"]), &[server]);
        assert_eq!(config["features.apps"], false);
        assert_eq!(config["mcp_servers.rea.enabled"], false);
        assert_eq!(config["mcp_servers.studio.enabled"], true);
        assert_eq!(config["mcp_servers.studio.env"]["TOKEN"], "v");
        assert_eq!(config["mcp_servers.studio.args"], json!(["--serve"]));
        assert_eq!(sandbox_policy()["type"], "workspaceWrite");
        assert_eq!(sandbox_policy()["networkAccess"], false);
    }
}
