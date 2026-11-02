//! GitHub sign-in (OAuth device flow), token storage, and the signed-in user's repositories.
//!
//! The device flow needs only an OAuth App client ID, which is public: it is either built in
//! (`GITCOVE_GITHUB_CLIENT_ID` at compile time) or pasted by the user once. The token lives in the
//! OS keychain, falling back to a private file when no keychain service answers.

use crate::git::error::{GitError, GitResult};
use crate::prs::parse_rfc3339;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{OnceLock, RwLock};
use std::time::Duration;

const BUILT_IN_CLIENT_ID: Option<&str> = option_env!("GITCOVE_GITHUB_CLIENT_ID");
const KEYRING_SERVICE: &str = "app.gitcove.desktop";
const KEYRING_USER: &str = "github.com";
const SCOPES: &str = "repo read:user read:org";

static CONFIG_DIR: OnceLock<PathBuf> = OnceLock::new();
static TOKEN: RwLock<Option<String>> = RwLock::new(None);

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct GithubAccount {
    pub login: String,
    pub name: Option<String>,
    pub avatar_url: Option<String>,
    pub html_url: Option<String>,
}

#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum ClientIdSource {
    BuiltIn,
    User,
}

#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum TokenStorage {
    Keychain,
    File,
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct GithubStatus {
    pub configured: bool,
    pub client_id_source: Option<ClientIdSource>,
    pub account: Option<GithubAccount>,
    pub storage: Option<TokenStorage>,
    /// True when the account shown came from the cache because GitHub couldn't be reached.
    pub offline: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct DeviceCode {
    pub device_code: String,
    pub user_code: String,
    pub verification_uri: String,
    pub expires_in: u64,
    pub interval: u64,
}

#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum LoginPollStatus {
    Pending,
    SlowDown,
    Expired,
    Denied,
    Ok,
    Error,
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct LoginPoll {
    pub status: LoginPollStatus,
    pub message: Option<String>,
    pub account: Option<GithubAccount>,
    /// New polling interval in seconds when GitHub asks to slow down.
    pub interval: Option<u64>,
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct GithubRepo {
    pub full_name: String,
    pub owner: String,
    pub name: String,
    pub private: bool,
    pub fork: bool,
    pub archived: bool,
    pub description: Option<String>,
    pub clone_url: String,
    pub ssh_url: String,
    pub html_url: String,
    pub pushed_at: i64,
    pub default_branch: Option<String>,
}

#[derive(Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct Config {
    client_id: Option<String>,
}

// ---------------------------------------------------------------------------------------
// Setup and storage
// ---------------------------------------------------------------------------------------

/// Called once at startup with the app's config directory; loads a saved token.
pub fn init(config_dir: PathBuf) {
    let _ = fs::create_dir_all(&config_dir);
    let _ = CONFIG_DIR.set(config_dir);
    if let Some((token, _)) = load_token() {
        set_cached_token(Some(token));
    }
}

fn config_dir() -> Option<&'static Path> {
    CONFIG_DIR.get().map(PathBuf::as_path)
}

fn read_config() -> Config {
    config_dir()
        .and_then(|d| fs::read_to_string(d.join("github.json")).ok())
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

fn client_id() -> Option<(String, ClientIdSource)> {
    if let Some(id) = BUILT_IN_CLIENT_ID.filter(|s| !s.trim().is_empty()) {
        return Some((id.trim().to_string(), ClientIdSource::BuiltIn));
    }
    read_config().client_id.filter(|s| !s.trim().is_empty()).map(|s| (s, ClientIdSource::User))
}

pub fn set_client_id(id: &str) -> GitResult<GithubStatus> {
    let id = id.trim();
    // GitHub OAuth App client IDs are 20 characters ("Ov23li…" or older hex ids).
    if id.len() < 16 || id.len() > 40 || !id.chars().all(|c| c.is_ascii_alphanumeric()) {
        return Err(GitError::other("That doesn't look like a GitHub OAuth App client ID. It's the short code shown under “Client ID” on the app's settings page."));
    }
    let dir = config_dir().ok_or_else(|| GitError::other("The app's settings folder isn't available."))?;
    let cfg = Config { client_id: Some(id.to_string()) };
    fs::write(dir.join("github.json"), serde_json::to_vec_pretty(&cfg).unwrap_or_default())
        .map_err(|e| GitError::other(format!("Couldn't save the client ID: {e}")))?;
    Ok(status())
}

/// The token for git's askpass and API calls, when signed in.
pub fn current_token() -> Option<String> {
    TOKEN.read().ok().and_then(|t| t.clone())
}

fn set_cached_token(token: Option<String>) {
    if let Ok(mut t) = TOKEN.write() {
        *t = token;
    }
}

fn token_file() -> Option<PathBuf> {
    config_dir().map(|d| d.join("github-token"))
}

fn account_file() -> Option<PathBuf> {
    config_dir().map(|d| d.join("github-account.json"))
}

fn keyring_entry() -> Option<keyring::Entry> {
    keyring::Entry::new(KEYRING_SERVICE, KEYRING_USER).ok()
}

fn save_token(token: &str) -> GitResult<TokenStorage> {
    if let Some(entry) = keyring_entry() {
        if entry.set_password(token).is_ok() && entry.get_password().ok().as_deref() == Some(token) {
            if let Some(f) = token_file() {
                let _ = fs::remove_file(f);
            }
            return Ok(TokenStorage::Keychain);
        }
    }
    let path = token_file().ok_or_else(|| GitError::other("The app's settings folder isn't available."))?;
    write_private(&path, token.as_bytes()).map_err(|e| GitError::other(format!("Couldn't save the sign-in: {e}")))?;
    Ok(TokenStorage::File)
}

fn load_token() -> Option<(String, TokenStorage)> {
    if let Some(entry) = keyring_entry() {
        if let Ok(t) = entry.get_password() {
            if !t.is_empty() {
                return Some((t, TokenStorage::Keychain));
            }
        }
    }
    let t = fs::read_to_string(token_file()?).ok()?;
    let t = t.trim().to_string();
    (!t.is_empty()).then_some((t, TokenStorage::File))
}

fn token_storage() -> Option<TokenStorage> {
    load_token().map(|(_, s)| s)
}

fn write_private(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    #[cfg(unix)]
    {
        use std::io::Write;
        use std::os::unix::fs::OpenOptionsExt;
        let mut f = fs::OpenOptions::new().create(true).truncate(true).write(true).mode(0o600).open(path)?;
        f.write_all(bytes)
    }
    #[cfg(not(unix))]
    {
        fs::write(path, bytes)
    }
}

pub fn sign_out() -> GitResult<()> {
    if let Some(entry) = keyring_entry() {
        let _ = entry.delete_credential();
    }
    for f in [token_file(), account_file()].into_iter().flatten() {
        let _ = fs::remove_file(f);
    }
    set_cached_token(None);
    Ok(())
}

// ---------------------------------------------------------------------------------------
// HTTP
// ---------------------------------------------------------------------------------------

fn agent() -> ureq::Agent {
    ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(12)))
        .http_status_as_error(false)
        .user_agent(concat!("gitcove/", env!("CARGO_PKG_VERSION")))
        .build()
        .into()
}

fn offline_error() -> GitError {
    GitError::new(
        crate::git::error::GitErrorKind::Network,
        "Couldn't reach github.com. Check your connection, then try again.",
        "",
    )
}

fn api_get(path: &str, token: &str) -> Result<(u16, Value, Option<String>), GitError> {
    let mut resp = agent()
        .get(&format!("https://api.github.com{path}"))
        .header("Accept", "application/vnd.github+json")
        .header("X-GitHub-Api-Version", "2022-11-28")
        .header("Authorization", &format!("Bearer {token}"))
        .call()
        .map_err(|_| offline_error())?;
    let status = resp.status().as_u16();
    let link = resp.headers().get("link").and_then(|v| v.to_str().ok()).map(str::to_string);
    let body: Value = resp.body_mut().read_json().unwrap_or(Value::Null);
    Ok((status, body, link))
}

fn parse_account(v: &Value) -> Option<GithubAccount> {
    Some(GithubAccount {
        login: v.get("login")?.as_str()?.to_string(),
        name: v.get("name").and_then(Value::as_str).filter(|s| !s.is_empty()).map(str::to_string),
        avatar_url: v.get("avatar_url").and_then(Value::as_str).map(str::to_string),
        html_url: v.get("html_url").and_then(Value::as_str).map(str::to_string),
    })
}

fn cache_account(a: &GithubAccount) {
    if let Some(f) = account_file() {
        let _ = fs::write(f, serde_json::to_vec(a).unwrap_or_default());
    }
}

fn cached_account() -> Option<GithubAccount> {
    serde_json::from_str(&fs::read_to_string(account_file()?).ok()?).ok()
}

// ---------------------------------------------------------------------------------------
// Commands
// ---------------------------------------------------------------------------------------

/// Who is signed in. A revoked token signs the user out; no network shows the cached account.
pub fn status() -> GithubStatus {
    let source = client_id().map(|(_, s)| s);
    let base = GithubStatus { configured: source.is_some(), client_id_source: source, account: None, storage: None, offline: false };
    let Some(token) = current_token() else { return base };
    match api_get("/user", &token) {
        Ok((200, body, _)) => {
            let account = parse_account(&body);
            if let Some(a) = &account {
                cache_account(a);
            }
            GithubStatus { account, storage: token_storage(), ..base }
        }
        Ok((401, _, _)) => {
            let _ = sign_out();
            base
        }
        _ => GithubStatus { account: cached_account(), storage: token_storage(), offline: true, ..base },
    }
}

pub fn start_login() -> GitResult<DeviceCode> {
    let (id, _) = client_id().ok_or_else(|| GitError::other("Add your GitHub OAuth App client ID first."))?;
    let mut resp = agent()
        .post("https://github.com/login/device/code")
        .header("Accept", "application/json")
        .send_form([("client_id", id.as_str()), ("scope", SCOPES)])
        .map_err(|_| offline_error())?;
    let status = resp.status().as_u16();
    let body: Value = resp.body_mut().read_json().unwrap_or(Value::Null);
    // An unknown client ID comes back as 404 {"error":"Not Found"}.
    if status == 404 {
        return Err(GitError::other(device_error_message("incorrect_client_credentials")));
    }
    if let Some(err) = body.get("error").and_then(Value::as_str) {
        return Err(GitError::other(device_error_message(err)));
    }
    if status >= 400 {
        return Err(GitError::other(format!("GitHub refused the sign-in request (HTTP {status}). Check the client ID.")));
    }
    serde_json::from_value(body).map_err(|_| GitError::other("GitHub sent a response this app couldn't read."))
}

fn device_error_message(err: &str) -> String {
    match err {
        "device_flow_disabled" => "Device sign-in is turned off for this OAuth App. Open the app's settings on GitHub, tick “Enable Device Flow”, and save.".into(),
        "incorrect_client_credentials" | "unauthorized_client" => {
            "GitHub doesn't recognise that client ID. Copy it again from the OAuth App's settings page.".into()
        }
        other => format!("GitHub refused the sign-in ({other})."),
    }
}

pub fn poll_login(device_code: &str) -> GitResult<LoginPoll> {
    let (id, _) = client_id().ok_or_else(|| GitError::other("Add your GitHub OAuth App client ID first."))?;
    let mut resp = agent()
        .post("https://github.com/login/oauth/access_token")
        .header("Accept", "application/json")
        .send_form([
            ("client_id", id.as_str()),
            ("device_code", device_code),
            ("grant_type", "urn:ietf:params:oauth:grant-type:device_code"),
        ])
        .map_err(|_| offline_error())?;
    let body: Value = resp.body_mut().read_json().unwrap_or(Value::Null);
    let poll = |status, message: Option<&str>| LoginPoll { status, message: message.map(str::to_string), account: None, interval: None };
    if let Some(err) = body.get("error").and_then(Value::as_str) {
        return Ok(match err {
            "authorization_pending" => poll(LoginPollStatus::Pending, None),
            "slow_down" => LoginPoll { interval: body.get("interval").and_then(Value::as_u64), ..poll(LoginPollStatus::SlowDown, None) },
            "expired_token" => poll(LoginPollStatus::Expired, Some("The code expired before it was approved. Start again to get a new one.")),
            "access_denied" => poll(LoginPollStatus::Denied, Some("Sign-in was cancelled on GitHub.")),
            other => LoginPoll { message: Some(device_error_message(other)), ..poll(LoginPollStatus::Error, None) },
        });
    }
    let Some(token) = body.get("access_token").and_then(Value::as_str) else {
        return Ok(poll(LoginPollStatus::Error, Some("GitHub sent a response this app couldn't read.")));
    };
    save_token(token)?;
    set_cached_token(Some(token.to_string()));
    let account = match api_get("/user", token) {
        Ok((200, body, _)) => parse_account(&body),
        _ => None,
    };
    if let Some(a) = &account {
        cache_account(a);
    }
    Ok(LoginPoll { account, ..poll(LoginPollStatus::Ok, None) })
}

fn next_link(link: Option<&str>) -> bool {
    link.map(|l| l.split(',').any(|part| part.contains("rel=\"next\""))).unwrap_or(false)
}

fn parse_repo(v: &Value) -> Option<GithubRepo> {
    Some(GithubRepo {
        full_name: v.get("full_name")?.as_str()?.to_string(),
        owner: v.get("owner")?.get("login")?.as_str()?.to_string(),
        name: v.get("name")?.as_str()?.to_string(),
        private: v.get("private").and_then(Value::as_bool).unwrap_or(false),
        fork: v.get("fork").and_then(Value::as_bool).unwrap_or(false),
        archived: v.get("archived").and_then(Value::as_bool).unwrap_or(false),
        description: v.get("description").and_then(Value::as_str).filter(|s| !s.is_empty()).map(str::to_string),
        clone_url: v.get("clone_url")?.as_str()?.to_string(),
        ssh_url: v.get("ssh_url").and_then(Value::as_str).unwrap_or_default().to_string(),
        html_url: v.get("html_url").and_then(Value::as_str).unwrap_or_default().to_string(),
        pushed_at: v.get("pushed_at").and_then(Value::as_str).map(parse_rfc3339).unwrap_or(0),
        default_branch: v.get("default_branch").and_then(Value::as_str).map(str::to_string),
    })
}

/// Every repository the user can push to or read, most recently pushed first (up to 1000).
pub fn list_repos() -> GitResult<Vec<GithubRepo>> {
    let token = current_token().ok_or_else(|| GitError::new(crate::git::error::GitErrorKind::Auth, "Sign in to GitHub to see your repositories.", ""))?;
    let mut out = Vec::new();
    for page in 1..=10 {
        let path = format!("/user/repos?per_page=100&sort=pushed&affiliation=owner,collaborator,organization_member&page={page}");
        let (status, body, link) = api_get(&path, &token)?;
        if status == 401 {
            let _ = sign_out();
            return Err(GitError::new(crate::git::error::GitErrorKind::Auth, "Your GitHub sign-in expired. Sign in again.", ""));
        }
        if status >= 400 {
            return Err(GitError::other(format!("GitHub couldn't list your repositories (HTTP {status}). Try again in a moment.")));
        }
        if let Some(items) = body.as_array() {
            out.extend(items.iter().filter_map(parse_repo));
        }
        if !next_link(link.as_deref()) {
            break;
        }
    }
    Ok(out)
}

/// A sensible parent folder for clones: ~/Documents when it exists, otherwise home.
pub fn default_clone_dir() -> String {
    let home = std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE")).map(PathBuf::from).unwrap_or_default();
    let docs = home.join("Documents");
    (if docs.is_dir() { docs } else { home }).to_string_lossy().into_owned()
}

// ---------------------------------------------------------------------------------------
// Git credentials
// ---------------------------------------------------------------------------------------

/// Environment for network git commands while signed in: git asks this app's own binary for
/// github.com credentials (see `askpass`), and the user's helpers are bypassed for github.com
/// only, so the token is never written into them. Other hosts are untouched.
pub fn git_env() -> Vec<(String, String)> {
    let (Some(token), Ok(exe)) = (current_token(), std::env::current_exe()) else { return vec![] };
    vec![
        ("GIT_ASKPASS".into(), exe.to_string_lossy().into_owned()),
        ("GITCOVE_ASKPASS".into(), "1".into()),
        ("GITCOVE_GITHUB_TOKEN".into(), token),
        ("GIT_CONFIG_COUNT".into(), "1".into()),
        ("GIT_CONFIG_KEY_0".into(), "credential.https://github.com.helper".into()),
        ("GIT_CONFIG_VALUE_0".into(), String::new()),
    ]
}

/// Entry point when git runs this binary as GIT_ASKPASS. Answers github.com prompts only.
pub fn askpass(prompt: &str) -> i32 {
    if !prompt.contains("github.com") {
        return 1;
    }
    let lower = prompt.trim_start().to_ascii_lowercase();
    if lower.starts_with("username") {
        println!("x-access-token");
        0
    } else if lower.starts_with("password") {
        match std::env::var("GITCOVE_GITHUB_TOKEN") {
            Ok(t) if !t.is_empty() => {
                println!("{t}");
                0
            }
            _ => 1,
        }
    } else {
        1
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn next_link_detects_more_pages() {
        let l = r#"<https://api.github.com/user/repos?page=2>; rel="next", <https://api.github.com/user/repos?page=5>; rel="last""#;
        assert!(next_link(Some(l)));
        assert!(!next_link(Some(r#"<https://api.github.com/user/repos?page=1>; rel="prev""#)));
        assert!(!next_link(None));
    }

    #[test]
    fn parses_repo_json() {
        let v: Value = serde_json::from_str(
            r#"{"full_name":"you/app","name":"app","owner":{"login":"you"},"private":true,"fork":false,"archived":false,
                "description":"","clone_url":"https://github.com/you/app.git","ssh_url":"git@github.com:you/app.git",
                "html_url":"https://github.com/you/app","pushed_at":"2026-10-30T12:00:00Z","default_branch":"main"}"#,
        )
        .unwrap();
        let r = parse_repo(&v).unwrap();
        assert_eq!(r.full_name, "you/app");
        assert!(r.private);
        assert_eq!(r.description, None);
        assert!(r.pushed_at > 1_700_000_000);
        let json = serde_json::to_value(&r).unwrap();
        assert!(json.get("cloneUrl").is_some() && json.get("pushedAt").is_some());
    }

    #[test]
    fn askpass_answers_github_only() {
        assert_eq!(askpass("Username for 'https://gitlab.com': "), 1);
        assert_eq!(askpass("Username for 'https://github.com': "), 0);
    }

    #[test]
    fn rejects_malformed_client_ids() {
        assert!(set_client_id("not an id!").is_err());
        assert!(set_client_id("short").is_err());
    }
}
