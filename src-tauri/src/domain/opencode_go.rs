//! Shared OpenCode Go service routes and local credential discovery.

use serde_json::Value;
use std::path::PathBuf;

/// OpenCode Go's account usage meters, scoped to the bearer key.
pub(super) const USAGE_URL: &str = "https://opencode.ai/inference/go/v1/usage";

/// Match only Go on the official service, including its older Go-specific alias.
pub(super) fn is_go_url(base: &str) -> bool {
    let Ok(url) = reqwest::Url::parse(base) else {
        return false;
    };
    url.scheme() == "https" && url.host_str() == Some("opencode.ai") && url.port().is_none() && ["/inference/go", "/zen/go"].iter().any(|prefix| url.path().strip_prefix(prefix).is_some_and(|suffix| suffix.is_empty() || suffix.starts_with('/')))
}

/// Select the current wire-level base for one of Pi's supported text APIs.
pub(super) fn inference_base(api: &str) -> Option<&'static str> {
    match api {
        "openai-completions" | "openai-responses" => Some("https://opencode.ai/inference/go/openai/v1"),
        "anthropic-messages" => Some("https://opencode.ai/inference/go/anthropic/v1"),
        "google-generative-ai" => Some("https://opencode.ai/inference/go/google/v1beta"),
        _ => None,
    }
}

/// Find a Go key, giving explicit Go credentials priority over the shared OpenCode environment key.
pub(super) fn local_api_key() -> Option<String> {
    let env_key = |name: &str| std::env::var(name).ok().map(|key| key.trim().to_string()).filter(|key| !key.is_empty());
    if let Some(key) = env_key("OPENCODE_GO_API_KEY") {
        return Some(key);
    }

    // Use the Go provider entry in each credential store, since other OpenCode entries may use another account.
    let pi_auth = crate::paths::pi_agent_root_dir().ok().map(|dir| dir.join("auth.json"));
    let opencode_auth = std::env::var_os("XDG_DATA_HOME").filter(|dir| !dir.is_empty()).map(PathBuf::from).or_else(|| dirs::home_dir().map(|home| home.join(".local/share"))).map(|dir| dir.join("opencode/auth.json"));
    for path in [pi_auth, opencode_auth].into_iter().flatten() {
        let Some(auth) = std::fs::read(path).ok().and_then(|data| serde_json::from_slice::<Value>(&data).ok()) else {
            continue;
        };
        if let Some(key) = auth.pointer("/opencode-go/key").and_then(Value::as_str).map(str::trim).filter(|key| !key.is_empty()) {
            return Some(key.to_string());
        }
    }
    env_key("OPENCODE_API_KEY")
}
