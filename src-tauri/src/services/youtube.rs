use std::{
    fs::{self, File},
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    path::{Path, PathBuf},
    sync::Mutex,
    thread,
    time::{Duration, Instant},
};

use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use chrono::{DateTime, Days, NaiveDate, SecondsFormat, Utc};
use keyring::Entry;
use reqwest::{
    blocking::{Client, Response},
    StatusCode,
};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use sha2::{Digest, Sha256};
use url::Url;
use uuid::Uuid;

use crate::domain::{
    AppError, AuthorizedChannel, YouTubeChannel, YouTubeConnectionPhase, YouTubeConnectionStatus,
    YouTubeDailyPerformance, YouTubePerformanceMetrics, YouTubePerformanceSnapshot,
    YouTubeProjectPerformance, YouTubeVideoCandidate,
};

const CATALOG_VERSION: u8 = 1;
const CATALOG_FILENAME: &str = "youtube-performance-v1.json";
const KEYRING_SERVICE: &str = "com.skulld.clipforge.youtube";
const KEYRING_ACCOUNT: &str = "refresh-token";
const AUTHORIZATION_ENDPOINT: &str = "https://accounts.google.com/o/oauth2/v2/auth";
const TOKEN_ENDPOINT: &str = "https://oauth2.googleapis.com/token";
const DATA_API_ENDPOINT: &str = "https://www.googleapis.com/youtube/v3";
const ANALYTICS_API_ENDPOINT: &str = "https://youtubeanalytics.googleapis.com/v2";
const YOUTUBE_SCOPES: &str = "https://www.googleapis.com/auth/youtube.readonly https://www.googleapis.com/auth/yt-analytics.readonly";
const MAX_RESPONSE_BYTES: u64 = 2 * 1024 * 1024;
const MAX_LINKS: usize = 100;
const MAX_RECENT_UPLOADS: usize = 25;
const OAUTH_TIMEOUT: Duration = Duration::from_secs(180);
const HTTP_TIMEOUT: Duration = Duration::from_secs(20);
const METRICS: &str = "engagedViews,views,estimatedMinutesWatched,averageViewDuration,averageViewPercentage,likes,comments,shares,subscribersGained,subscribersLost";

#[derive(Debug, Clone, Copy)]
enum GoogleApiContext {
    OAuth,
    YouTubeData,
    YouTubeAnalytics,
}

#[derive(Debug, Clone)]
struct OAuthConfig {
    client_id: String,
    client_secret: Option<String>,
}

impl OAuthConfig {
    fn from_environment() -> Option<Self> {
        let client_id = std::env::var("SKCF_YOUTUBE_CLIENT_ID")
            .ok()
            .or_else(|| option_env!("SKCF_YOUTUBE_CLIENT_ID").map(str::to_owned))
            .map(|value| value.trim().to_owned())
            .filter(|value| !value.is_empty())?;
        let client_secret = std::env::var("SKCF_YOUTUBE_CLIENT_SECRET")
            .ok()
            .or_else(|| option_env!("SKCF_YOUTUBE_CLIENT_SECRET").map(str::to_owned))
            .map(|value| value.trim().to_owned())
            .filter(|value| !value.is_empty());
        Some(Self {
            client_id,
            client_secret,
        })
    }
}

#[derive(Debug, Clone)]
struct CachedAccessToken {
    value: String,
    expires_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct StoredConnection {
    channel: YouTubeChannel,
    uploads_playlist_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct YouTubeCatalog {
    version: u8,
    connection: Option<StoredConnection>,
    links: Vec<YouTubeProjectPerformance>,
}

impl Default for YouTubeCatalog {
    fn default() -> Self {
        Self {
            version: CATALOG_VERSION,
            connection: None,
            links: Vec::new(),
        }
    }
}

pub struct YouTubePerformanceService {
    catalog_path: PathBuf,
    access_token: Mutex<Option<CachedAccessToken>>,
    connection_phase: Mutex<YouTubeConnectionPhase>,
    operation_lock: Mutex<()>,
}

impl YouTubePerformanceService {
    pub fn new(app_local_data_dir: PathBuf) -> Self {
        Self {
            catalog_path: app_local_data_dir.join(CATALOG_FILENAME),
            access_token: Mutex::new(None),
            connection_phase: Mutex::new(YouTubeConnectionPhase::Disconnected),
            operation_lock: Mutex::new(()),
        }
    }

    pub fn connection_status(&self) -> Result<YouTubeConnectionStatus, AppError> {
        let catalog = self.load_catalog()?;
        let last_synced_at = catalog
            .links
            .iter()
            .filter_map(|link| {
                link.performance
                    .as_ref()
                    .map(|performance| performance.synced_at.as_str())
            })
            .max()
            .map(str::to_owned);
        let authenticated = catalog.connection.is_some();
        let connection_phase = if authenticated {
            YouTubeConnectionPhase::Connected
        } else {
            self.connection_phase()?
        };
        Ok(YouTubeConnectionStatus {
            configured: OAuthConfig::from_environment().is_some(),
            authenticated,
            connection_phase,
            channel: catalog.connection.map(|connection| connection.channel),
            last_synced_at,
        })
    }

    pub fn connect(&self) -> Result<YouTubeConnectionStatus, AppError> {
        let _operation = self.lock_operations()?;
        let config = configured_oauth()?;
        self.set_connection_phase(YouTubeConnectionPhase::AwaitingBrowser)?;
        let result = (|| {
            let listener = TcpListener::bind(("127.0.0.1", 0)).map_err(|_| {
                AppError::network(
                    "The local OAuth callback could not start. Check local firewall settings.",
                )
            })?;
            let redirect_uri = format!(
                "http://127.0.0.1:{}/oauth/callback",
                listener
                    .local_addr()
                    .map_err(|_| AppError::network("The OAuth callback address is unavailable."))?
                    .port()
            );
            let state = Uuid::new_v4().to_string();
            let code_verifier = format!("{}{}", Uuid::new_v4().simple(), Uuid::new_v4().simple());
            let code_challenge = URL_SAFE_NO_PAD.encode(Sha256::digest(code_verifier.as_bytes()));
            let authorization_url =
                build_authorization_url(&config.client_id, &redirect_uri, &state, &code_challenge)?;

            webbrowser::open(authorization_url.as_str()).map_err(|_| {
                AppError::network(
                    "The system browser could not open. Set a default browser and retry.",
                )
            })?;
            let authorization_code = receive_oauth_callback(listener, &state, OAUTH_TIMEOUT)?;
            self.set_connection_phase(YouTubeConnectionPhase::ExchangingToken)?;
            let token = exchange_authorization_code(
                &self.http_client()?,
                &config,
                &authorization_code,
                &redirect_uri,
                &code_verifier,
            )?;
            let refresh_token = token.refresh_token.ok_or_else(|| {
                AppError::auth_required(
                    "Google did not issue offline access. Reconnect and approve both read-only scopes.",
                )
            })?;
            self.set_connection_phase(YouTubeConnectionPhase::LoadingChannel)?;
            let channel = fetch_authorized_channel(&self.http_client()?, &token.access_token)?;
            save_refresh_token(&refresh_token)?;
            let mut catalog = self.load_catalog()?;
            catalog.connection = Some(StoredConnection {
                channel: channel.channel.clone(),
                uploads_playlist_id: channel.uploads_playlist_id,
            });
            catalog.links.clear();
            if let Err(error) = self.save_catalog(&catalog) {
                let _ = delete_refresh_token();
                return Err(error);
            }
            self.cache_access_token(&token.access_token, token.expires_in)?;
            self.set_connection_phase(YouTubeConnectionPhase::Connected)?;

            Ok(YouTubeConnectionStatus {
                configured: true,
                authenticated: true,
                connection_phase: YouTubeConnectionPhase::Connected,
                channel: Some(channel.channel),
                last_synced_at: None,
            })
        })();
        if result.is_err() {
            let _ = self.set_connection_phase(YouTubeConnectionPhase::Failed);
        }
        result
    }

    pub fn disconnect(&self) -> Result<YouTubeConnectionStatus, AppError> {
        let _operation = self.lock_operations()?;
        delete_refresh_token()?;
        *self
            .access_token
            .lock()
            .map_err(|_| AppError::internal("YouTube credential state is unavailable."))? = None;
        for path in [
            self.catalog_path.clone(),
            self.catalog_path.with_extension("json.bak"),
            self.catalog_path.with_extension("json.tmp"),
        ] {
            if path.exists() {
                fs::remove_file(path).map_err(|_| {
                    AppError::io(
                        "YouTube performance data could not be cleared.",
                        "Close other copies of the app and retry.",
                    )
                })?;
            }
        }
        self.set_connection_phase(YouTubeConnectionPhase::Disconnected)?;
        Ok(YouTubeConnectionStatus {
            configured: OAuthConfig::from_environment().is_some(),
            authenticated: false,
            connection_phase: YouTubeConnectionPhase::Disconnected,
            channel: None,
            last_synced_at: None,
        })
    }

    pub fn list_recent_uploads(&self) -> Result<Vec<YouTubeVideoCandidate>, AppError> {
        let _operation = self.lock_operations()?;
        let config = configured_oauth()?;
        let catalog = self.load_catalog()?;
        let connection = require_connection(&catalog)?;
        let access_token = self.valid_access_token(&config)?;
        fetch_recent_uploads(
            &self.http_client()?,
            &access_token,
            &connection.uploads_playlist_id,
        )
    }

    pub fn link_project(
        &self,
        project_id: &str,
        project_name: &str,
        video_id_or_url: &str,
    ) -> Result<YouTubeProjectPerformance, AppError> {
        let _operation = self.lock_operations()?;
        validate_project_reference(project_id, project_name)?;
        let video_id = parse_video_id(video_id_or_url)?;
        let config = configured_oauth()?;
        let mut catalog = self.load_catalog()?;
        let connection = require_connection(&catalog)?;
        let access_token = self.valid_access_token(&config)?;
        let video = fetch_video(
            &self.http_client()?,
            &access_token,
            &connection.channel.channel_id,
            &video_id,
        )?;
        let link = YouTubeProjectPerformance {
            project_id: project_id.to_owned(),
            project_name: project_name.trim().to_owned(),
            video_id: video.video_id,
            video_title: video.title,
            published_at: video.published_at,
            linked_at: now_rfc3339(),
            performance: None,
        };
        if let Some(index) = catalog
            .links
            .iter()
            .position(|candidate| candidate.project_id == project_id)
        {
            catalog.links[index] = link.clone();
        } else {
            if catalog.links.len() >= MAX_LINKS {
                catalog
                    .links
                    .sort_by(|left, right| left.linked_at.cmp(&right.linked_at));
                catalog.links.remove(0);
            }
            catalog.links.push(link.clone());
        }
        self.save_catalog(&catalog)?;
        Ok(link)
    }

    pub fn list_performance(&self) -> Result<Vec<YouTubeProjectPerformance>, AppError> {
        let _operation = self.lock_operations()?;
        let mut links = self.load_catalog()?.links;
        links.sort_by(|left, right| right.published_at.cmp(&left.published_at));
        Ok(links)
    }

    pub fn sync_performance(
        &self,
        project_id: Option<&str>,
    ) -> Result<Vec<YouTubeProjectPerformance>, AppError> {
        let _operation = self.lock_operations()?;
        if let Some(project_id) = project_id {
            Uuid::parse_str(project_id)
                .map_err(|_| AppError::invalid_argument("The project ID is invalid."))?;
        }
        let config = configured_oauth()?;
        let mut catalog = self.load_catalog()?;
        let connection = require_connection(&catalog)?;
        let access_token = self.valid_access_token(&config)?;
        let client = self.http_client()?;
        let channel_id = connection.channel.channel_id;
        let mut matched = false;
        for link in &mut catalog.links {
            if project_id.map_or(true, |wanted| wanted == link.project_id) {
                matched = true;
                link.performance = Some(fetch_performance_snapshot(
                    &client,
                    &access_token,
                    &channel_id,
                    &link.video_id,
                    &link.published_at,
                )?);
            }
        }
        if project_id.is_some() && !matched {
            return Err(AppError::invalid_argument(
                "Link this project to a YouTube upload before refreshing performance.",
            ));
        }
        self.save_catalog(&catalog)?;
        catalog
            .links
            .sort_by(|left, right| right.published_at.cmp(&left.published_at));
        Ok(catalog.links)
    }

    fn valid_access_token(&self, config: &OAuthConfig) -> Result<String, AppError> {
        {
            let guard = self
                .access_token
                .lock()
                .map_err(|_| AppError::internal("YouTube credential state is unavailable."))?;
            if let Some(token) = guard.as_ref() {
                if token.expires_at > Utc::now() + chrono::Duration::seconds(60) {
                    return Ok(token.value.clone());
                }
            }
        }

        let refresh_token = load_refresh_token()?;
        let token = refresh_access_token(&self.http_client()?, config, &refresh_token)?;
        self.cache_access_token(&token.access_token, token.expires_in)?;
        Ok(token.access_token)
    }

    fn cache_access_token(&self, access_token: &str, expires_in: u64) -> Result<(), AppError> {
        let expires_in = i64::try_from(expires_in.min(86_400))
            .map_err(|_| AppError::internal("YouTube token expiry is invalid."))?;
        *self
            .access_token
            .lock()
            .map_err(|_| AppError::internal("YouTube credential state is unavailable."))? =
            Some(CachedAccessToken {
                value: access_token.to_owned(),
                expires_at: Utc::now() + chrono::Duration::seconds(expires_in),
            });
        Ok(())
    }

    fn http_client(&self) -> Result<Client, AppError> {
        Client::builder()
            .timeout(HTTP_TIMEOUT)
            .user_agent("Skulld-Clip-Forge/0.1")
            .build()
            .map_err(|_| AppError::network("The secure HTTP client could not start."))
    }

    fn connection_phase(&self) -> Result<YouTubeConnectionPhase, AppError> {
        self.connection_phase
            .lock()
            .map(|phase| *phase)
            .map_err(|_| AppError::internal("YouTube connection state is unavailable."))
    }

    fn set_connection_phase(&self, phase: YouTubeConnectionPhase) -> Result<(), AppError> {
        *self
            .connection_phase
            .lock()
            .map_err(|_| AppError::internal("YouTube connection state is unavailable."))? = phase;
        Ok(())
    }

    fn lock_operations(&self) -> Result<std::sync::MutexGuard<'_, ()>, AppError> {
        self.operation_lock
            .lock()
            .map_err(|_| AppError::internal("YouTube performance state is unavailable."))
    }

    fn load_catalog(&self) -> Result<YouTubeCatalog, AppError> {
        if !self.catalog_path.exists() {
            return Ok(YouTubeCatalog::default());
        }
        let bytes = fs::read(&self.catalog_path).map_err(|_| {
            AppError::io(
                "YouTube performance data could not be read.",
                "Check app-data permissions and retry.",
            )
        })?;
        if bytes.len() as u64 > MAX_RESPONSE_BYTES {
            return Err(AppError::io(
                "YouTube performance data is invalid.",
                "The local performance file exceeds the supported size.",
            ));
        }
        let catalog: YouTubeCatalog = serde_json::from_slice(&bytes).map_err(|_| {
            AppError::io(
                "YouTube performance data is invalid.",
                "Reconnect the channel to rebuild local performance data.",
            )
        })?;
        if catalog.version != CATALOG_VERSION || catalog.links.len() > MAX_LINKS {
            return Err(AppError::io(
                "YouTube performance data is unsupported.",
                "Update the app or reconnect the channel.",
            ));
        }
        Ok(catalog)
    }

    fn save_catalog(&self, catalog: &YouTubeCatalog) -> Result<(), AppError> {
        let parent = self
            .catalog_path
            .parent()
            .ok_or_else(|| AppError::internal("YouTube data path has no parent."))?;
        fs::create_dir_all(parent).map_err(|_| {
            AppError::io(
                "YouTube performance data could not be saved.",
                "Check app-data permissions and retry.",
            )
        })?;
        let bytes = serde_json::to_vec_pretty(catalog)
            .map_err(|_| AppError::internal("YouTube performance data could not be serialized."))?;
        write_atomic(&self.catalog_path, &bytes)
    }
}

fn configured_oauth() -> Result<OAuthConfig, AppError> {
    OAuthConfig::from_environment().ok_or_else(|| {
        AppError::integration_unavailable(
            "Set SKCF_YOUTUBE_CLIENT_ID for a Google OAuth desktop client, then restart the app.",
        )
    })
}

fn require_connection(catalog: &YouTubeCatalog) -> Result<StoredConnection, AppError> {
    catalog.connection.clone().ok_or_else(|| {
        AppError::auth_required("Connect the channel and approve both read-only YouTube scopes.")
    })
}

fn validate_project_reference(project_id: &str, project_name: &str) -> Result<(), AppError> {
    Uuid::parse_str(project_id)
        .map_err(|_| AppError::invalid_argument("The project ID is invalid."))?;
    let name = project_name.trim();
    if name.is_empty() || name.chars().count() > 120 {
        return Err(AppError::invalid_argument(
            "The project name must contain 1 to 120 characters.",
        ));
    }
    Ok(())
}

fn build_authorization_url(
    client_id: &str,
    redirect_uri: &str,
    state: &str,
    code_challenge: &str,
) -> Result<Url, AppError> {
    let mut url = Url::parse(AUTHORIZATION_ENDPOINT)
        .map_err(|_| AppError::internal("The YouTube authorization endpoint is invalid."))?;
    url.query_pairs_mut()
        .append_pair("client_id", client_id)
        .append_pair("redirect_uri", redirect_uri)
        .append_pair("response_type", "code")
        .append_pair("scope", YOUTUBE_SCOPES)
        .append_pair("access_type", "offline")
        .append_pair("include_granted_scopes", "true")
        .append_pair("prompt", "consent")
        .append_pair("state", state)
        .append_pair("code_challenge", code_challenge)
        .append_pair("code_challenge_method", "S256");
    Ok(url)
}

fn receive_oauth_callback(
    listener: TcpListener,
    expected_state: &str,
    timeout: Duration,
) -> Result<String, AppError> {
    listener
        .set_nonblocking(true)
        .map_err(|_| AppError::network("The OAuth callback could not be secured."))?;
    let deadline = Instant::now() + timeout;
    loop {
        if Instant::now() >= deadline {
            return Err(AppError::auth_required(
                "YouTube authorization timed out. Retry and finish in the browser.",
            ));
        }
        match listener.accept() {
            Ok((stream, _)) => {
                if let Some(code) = handle_oauth_callback_stream(stream, expected_state)? {
                    return Ok(code);
                }
            }
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                thread::sleep(Duration::from_millis(50));
            }
            Err(_) => {
                return Err(AppError::network(
                    "The local OAuth callback stopped before authorization completed.",
                ));
            }
        }
    }
}

fn handle_oauth_callback_stream(
    mut stream: TcpStream,
    expected_state: &str,
) -> Result<Option<String>, AppError> {
    stream
        .set_read_timeout(Some(Duration::from_secs(2)))
        .map_err(|_| AppError::network("The OAuth callback could not be read."))?;
    let request = read_http_request_head(&mut stream)?;
    let Some(target) = request
        .lines()
        .next()
        .and_then(|line| line.split_whitespace().nth(1))
    else {
        write_callback_response(&mut stream, 400, false);
        return Ok(None);
    };
    let code = parse_oauth_callback_target(target, expected_state);
    match &code {
        Ok(Some(_)) => write_callback_response(&mut stream, 200, true),
        Ok(None) => write_callback_response(&mut stream, 404, false),
        Err(_) => write_callback_response(&mut stream, 400, false),
    }
    code
}

fn read_http_request_head(reader: &mut impl Read) -> Result<String, AppError> {
    let mut bytes = Vec::with_capacity(1024);
    let mut chunk = [0_u8; 512];
    loop {
        match reader.read(&mut chunk) {
            Ok(0) => break,
            Ok(count) => {
                bytes.extend_from_slice(&chunk[..count]);
                if bytes.windows(4).any(|window| window == b"\r\n\r\n") {
                    break;
                }
                if bytes.len() >= 8 * 1024 {
                    return Err(AppError::auth_required(
                        "Google returned an oversized authorization response.",
                    ));
                }
            }
            Err(error) if error.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(error)
                if matches!(
                    error.kind(),
                    std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
                ) =>
            {
                break;
            }
            Err(_) => return Err(AppError::network("The OAuth callback was incomplete.")),
        }
    }
    String::from_utf8(bytes)
        .map_err(|_| AppError::auth_required("Google returned an invalid authorization response."))
}

fn parse_oauth_callback_target(
    target: &str,
    expected_state: &str,
) -> Result<Option<String>, AppError> {
    let callback_url = Url::parse(&format!("http://127.0.0.1{target}")).map_err(|_| {
        AppError::auth_required("Google returned an invalid authorization response.")
    })?;
    if callback_url.path() != "/oauth/callback" {
        return Ok(None);
    }
    let pairs: std::collections::HashMap<String, String> =
        callback_url.query_pairs().into_owned().collect();
    if pairs.get("state").map(String::as_str) != Some(expected_state) {
        return Err(AppError::auth_required(
            "The OAuth state check failed. Close the browser tab and retry.",
        ));
    }
    if pairs.contains_key("error") {
        return Err(AppError::auth_required(
            "YouTube access was not approved. Both read-only scopes are required.",
        ));
    }
    let code = pairs
        .get("code")
        .cloned()
        .ok_or_else(|| AppError::auth_required("Google did not return an authorization code."))?;
    Ok(Some(code))
}

fn write_callback_response(stream: &mut TcpStream, status: u16, success: bool) {
    let (status_text, body) = if success {
        (
            "200 OK",
            "<!doctype html><meta charset=\"utf-8\"><title>Skull'd Clip Forge</title><body style=\"font-family:system-ui;background:#0b0b0d;color:#f4f0e8;padding:48px\"><h1>Channel connected</h1><p>Return to Skull'd Clip Forge. You can close this tab.</p></body>",
        )
    } else {
        (
            match status {
                404 => "404 Not Found",
                _ => "400 Bad Request",
            },
            "<!doctype html><meta charset=\"utf-8\"><title>Skull'd Clip Forge</title><body style=\"font-family:system-ui;background:#0b0b0d;color:#f4f0e8;padding:48px\"><h1>Connection not completed</h1><p>Return to Skull'd Clip Forge and retry.</p></body>",
        )
    };
    let response = format!(
        "HTTP/1.1 {status_text}\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    let _ = stream.write_all(response.as_bytes());
    let _ = stream.flush();
}

#[derive(Debug, Deserialize)]
struct TokenResponse {
    access_token: String,
    expires_in: u64,
    refresh_token: Option<String>,
}

fn exchange_authorization_code(
    client: &Client,
    config: &OAuthConfig,
    code: &str,
    redirect_uri: &str,
    code_verifier: &str,
) -> Result<TokenResponse, AppError> {
    let mut form = vec![
        ("client_id", config.client_id.as_str()),
        ("code", code),
        ("code_verifier", code_verifier),
        ("grant_type", "authorization_code"),
        ("redirect_uri", redirect_uri),
    ];
    if let Some(secret) = config.client_secret.as_deref() {
        form.push(("client_secret", secret));
    }
    let response = client
        .post(TOKEN_ENDPOINT)
        .form(&form)
        .send()
        .map_err(|_| AppError::network("The OAuth token exchange failed. Retry connection."))?;
    decode_google_response(
        response,
        "YouTube authorization failed.",
        GoogleApiContext::OAuth,
    )
}

fn refresh_access_token(
    client: &Client,
    config: &OAuthConfig,
    refresh_token: &str,
) -> Result<TokenResponse, AppError> {
    let mut form = vec![
        ("client_id", config.client_id.as_str()),
        ("refresh_token", refresh_token),
        ("grant_type", "refresh_token"),
    ];
    if let Some(secret) = config.client_secret.as_deref() {
        form.push(("client_secret", secret));
    }
    let response = client
        .post(TOKEN_ENDPOINT)
        .form(&form)
        .send()
        .map_err(|_| AppError::network("The YouTube session could not be refreshed."))?;
    decode_google_response(
        response,
        "Reconnect the YouTube channel.",
        GoogleApiContext::OAuth,
    )
}

fn save_refresh_token(refresh_token: &str) -> Result<(), AppError> {
    keyring_entry()?.set_password(refresh_token).map_err(|_| {
        AppError::io(
            "YouTube credentials could not be saved.",
            "Unlock the operating-system credential store and retry.",
        )
    })
}

fn load_refresh_token() -> Result<String, AppError> {
    keyring_entry()?.get_password().map_err(|_| {
        AppError::auth_required(
            "The saved YouTube credential is unavailable. Reconnect the channel.",
        )
    })
}

fn delete_refresh_token() -> Result<(), AppError> {
    match keyring_entry()?.delete_credential() {
        Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
        Err(_) => Err(AppError::io(
            "YouTube credentials could not be removed.",
            "Unlock the operating-system credential store and retry.",
        )),
    }
}

fn keyring_entry() -> Result<Entry, AppError> {
    Entry::new(KEYRING_SERVICE, KEYRING_ACCOUNT).map_err(|_| {
        AppError::io(
            "The operating-system credential store is unavailable.",
            "Unlock the credential store and retry.",
        )
    })
}

#[derive(Debug, Deserialize)]
struct ChannelListResponse {
    #[serde(default)]
    items: Vec<ChannelResource>,
}

#[derive(Debug, Deserialize)]
struct ChannelResource {
    id: String,
    snippet: ChannelSnippet,
    #[serde(rename = "contentDetails")]
    content_details: ChannelContentDetails,
}

#[derive(Debug, Deserialize)]
struct ChannelSnippet {
    title: String,
}

#[derive(Debug, Deserialize)]
struct ChannelContentDetails {
    #[serde(rename = "relatedPlaylists")]
    related_playlists: RelatedPlaylists,
}

#[derive(Debug, Deserialize)]
struct RelatedPlaylists {
    uploads: String,
}

fn fetch_authorized_channel(
    client: &Client,
    access_token: &str,
) -> Result<AuthorizedChannel, AppError> {
    let response = client
        .get(format!("{DATA_API_ENDPOINT}/channels"))
        .bearer_auth(access_token)
        .query(&[("part", "snippet,contentDetails"), ("mine", "true")])
        .send()
        .map_err(|_| AppError::network("The authorized YouTube channel could not be loaded."))?;
    let mut channels: ChannelListResponse = decode_google_response(
        response,
        "The authorized YouTube channel is unavailable.",
        GoogleApiContext::YouTubeData,
    )?;
    let channel = channels.items.pop().ok_or_else(|| {
        AppError::youtube_api(
            "The Google account does not expose an authorized YouTube channel.",
            false,
        )
    })?;
    Ok(AuthorizedChannel {
        channel: YouTubeChannel {
            channel_id: channel.id,
            title: channel.snippet.title,
        },
        uploads_playlist_id: channel.content_details.related_playlists.uploads,
    })
}

#[derive(Debug, Deserialize)]
struct PlaylistItemsResponse {
    #[serde(default)]
    items: Vec<PlaylistItemResource>,
}

#[derive(Debug, Deserialize)]
struct PlaylistItemResource {
    snippet: PlaylistItemSnippet,
    #[serde(rename = "contentDetails")]
    content_details: PlaylistItemContentDetails,
}

#[derive(Debug, Deserialize)]
struct PlaylistItemSnippet {
    title: String,
    #[serde(rename = "publishedAt")]
    published_at: String,
}

#[derive(Debug, Deserialize)]
struct PlaylistItemContentDetails {
    #[serde(rename = "videoId")]
    video_id: String,
}

fn fetch_recent_uploads(
    client: &Client,
    access_token: &str,
    playlist_id: &str,
) -> Result<Vec<YouTubeVideoCandidate>, AppError> {
    let response = client
        .get(format!("{DATA_API_ENDPOINT}/playlistItems"))
        .bearer_auth(access_token)
        .query(&[
            ("part", "snippet,contentDetails"),
            ("playlistId", playlist_id),
            ("maxResults", &MAX_RECENT_UPLOADS.to_string()),
        ])
        .send()
        .map_err(|_| AppError::network("Recent YouTube uploads could not be loaded."))?;
    let uploads: PlaylistItemsResponse = decode_google_response(
        response,
        "Recent YouTube uploads are unavailable.",
        GoogleApiContext::YouTubeData,
    )?;
    Ok(uploads
        .items
        .into_iter()
        .map(|item| YouTubeVideoCandidate {
            video_id: item.content_details.video_id,
            title: item.snippet.title,
            published_at: item.snippet.published_at,
        })
        .collect())
}

#[derive(Debug, Deserialize)]
struct VideoListResponse {
    #[serde(default)]
    items: Vec<VideoResource>,
}

#[derive(Debug, Deserialize)]
struct VideoResource {
    id: String,
    snippet: VideoSnippet,
}

#[derive(Debug, Deserialize)]
struct VideoSnippet {
    title: String,
    #[serde(rename = "publishedAt")]
    published_at: String,
    #[serde(rename = "channelId")]
    channel_id: String,
}

fn fetch_video(
    client: &Client,
    access_token: &str,
    connected_channel_id: &str,
    video_id: &str,
) -> Result<YouTubeVideoCandidate, AppError> {
    let response = client
        .get(format!("{DATA_API_ENDPOINT}/videos"))
        .bearer_auth(access_token)
        .query(&[("part", "snippet"), ("id", video_id)])
        .send()
        .map_err(|_| AppError::network("The selected YouTube video could not be loaded."))?;
    let mut videos: VideoListResponse = decode_google_response(
        response,
        "The selected YouTube video is unavailable.",
        GoogleApiContext::YouTubeData,
    )?;
    let video = videos.items.pop().ok_or_else(|| {
        AppError::youtube_api(
            "No accessible YouTube video matches that URL or video ID.",
            false,
        )
    })?;
    video_candidate_for_channel(video, connected_channel_id)
}

fn video_candidate_for_channel(
    video: VideoResource,
    connected_channel_id: &str,
) -> Result<YouTubeVideoCandidate, AppError> {
    if video.snippet.channel_id != connected_channel_id {
        return Err(AppError::youtube_api(
            "The selected video does not belong to the connected channel.",
            false,
        ));
    }
    Ok(YouTubeVideoCandidate {
        video_id: video.id,
        title: video.snippet.title,
        published_at: video.snippet.published_at,
    })
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AnalyticsResponse {
    #[serde(default)]
    start_date: Option<String>,
    #[serde(default)]
    end_date: Option<String>,
    #[serde(default)]
    column_headers: Vec<AnalyticsColumn>,
    #[serde(default)]
    rows: Vec<Vec<serde_json::Value>>,
}

#[derive(Debug, Deserialize)]
struct AnalyticsColumn {
    name: String,
}

fn fetch_performance_snapshot(
    client: &Client,
    access_token: &str,
    channel_id: &str,
    video_id: &str,
    published_at: &str,
) -> Result<YouTubePerformanceSnapshot, AppError> {
    let published = DateTime::parse_from_rfc3339(published_at)
        .map_err(|_| AppError::youtube_api("YouTube returned an invalid publish date.", false))?
        .date_naive();
    let today = Utc::now().date_naive();
    let end_date = today.checked_sub_days(Days::new(1)).unwrap_or(today);
    if published > end_date {
        return Ok(YouTubePerformanceSnapshot {
            start_date: published.to_string(),
            end_date: published.to_string(),
            synced_at: now_rfc3339(),
            metrics: YouTubePerformanceMetrics::default(),
            daily: Vec::new(),
        });
    }

    let ids = format!("channel=={channel_id}");
    let filter = format!("video=={video_id}");
    let start_date = published.to_string();
    let end_date_string = end_date.to_string();
    let aggregate = query_analytics(
        client,
        access_token,
        &ids,
        &start_date,
        &end_date_string,
        &filter,
        None,
    )?;
    let daily = query_analytics(
        client,
        access_token,
        &ids,
        &start_date,
        &end_date_string,
        &filter,
        Some("day"),
    )?;
    Ok(YouTubePerformanceSnapshot {
        start_date: aggregate.start_date.clone().unwrap_or(start_date),
        end_date: aggregate.end_date.clone().unwrap_or(end_date_string),
        synced_at: now_rfc3339(),
        metrics: analytics_totals(&aggregate)?,
        daily: analytics_daily(&daily)?,
    })
}

fn query_analytics(
    client: &Client,
    access_token: &str,
    ids: &str,
    start_date: &str,
    end_date: &str,
    filter: &str,
    dimensions: Option<&str>,
) -> Result<AnalyticsResponse, AppError> {
    let mut query = vec![
        ("ids", ids),
        ("startDate", start_date),
        ("endDate", end_date),
        ("metrics", METRICS),
        ("filters", filter),
    ];
    if let Some(dimensions) = dimensions {
        query.push(("dimensions", dimensions));
        query.push(("sort", "day"));
    }
    let response = client
        .get(format!("{ANALYTICS_API_ENDPOINT}/reports"))
        .bearer_auth(access_token)
        .query(&query)
        .send()
        .map_err(|_| AppError::network("YouTube Analytics could not be reached."))?;
    decode_google_response(
        response,
        "YouTube Analytics rejected the performance query.",
        GoogleApiContext::YouTubeAnalytics,
    )
}

fn analytics_totals(response: &AnalyticsResponse) -> Result<YouTubePerformanceMetrics, AppError> {
    let Some(row) = response.rows.first() else {
        return Ok(YouTubePerformanceMetrics::default());
    };
    metrics_from_row(&response.column_headers, row)
}

fn analytics_daily(response: &AnalyticsResponse) -> Result<Vec<YouTubeDailyPerformance>, AppError> {
    let day_index = response
        .column_headers
        .iter()
        .position(|column| column.name == "day")
        .ok_or_else(|| AppError::youtube_api("YouTube omitted the daily report date.", true))?;
    response
        .rows
        .iter()
        .map(|row| {
            let date = row
                .get(day_index)
                .and_then(serde_json::Value::as_str)
                .filter(|value| NaiveDate::parse_from_str(value, "%Y-%m-%d").is_ok())
                .ok_or_else(|| {
                    AppError::youtube_api("YouTube returned an invalid daily report date.", true)
                })?;
            Ok(YouTubeDailyPerformance {
                date: date.to_owned(),
                metrics: metrics_from_row(&response.column_headers, row)?,
            })
        })
        .collect()
}

fn metrics_from_row(
    headers: &[AnalyticsColumn],
    row: &[serde_json::Value],
) -> Result<YouTubePerformanceMetrics, AppError> {
    let value = |name: &str| -> Result<f64, AppError> {
        let index = headers
            .iter()
            .position(|column| column.name == name)
            .ok_or_else(|| {
                AppError::youtube_api("YouTube returned an incomplete performance report.", true)
            })?;
        row.get(index)
            .and_then(serde_json::Value::as_f64)
            .ok_or_else(|| {
                AppError::youtube_api("YouTube returned an invalid performance value.", true)
            })
    };
    Ok(YouTubePerformanceMetrics {
        engaged_views: value("engagedViews")?,
        views: value("views")?,
        estimated_minutes_watched: value("estimatedMinutesWatched")?,
        average_view_duration_seconds: value("averageViewDuration")?,
        average_view_percentage: value("averageViewPercentage")?,
        likes: value("likes")?,
        comments: value("comments")?,
        shares: value("shares")?,
        subscribers_gained: value("subscribersGained")?,
        subscribers_lost: value("subscribersLost")?,
    })
}

fn decode_google_response<T: DeserializeOwned>(
    response: Response,
    safe_detail: &str,
    api: GoogleApiContext,
) -> Result<T, AppError> {
    let status = response.status();
    let content_length = response.content_length();
    decode_google_body(status, content_length, response, safe_detail, api)
}

fn decode_google_body<T: DeserializeOwned, R: Read>(
    status: StatusCode,
    content_length: Option<u64>,
    reader: R,
    safe_detail: &str,
    api: GoogleApiContext,
) -> Result<T, AppError> {
    if let Some(length) = content_length {
        if length > MAX_RESPONSE_BYTES {
            return Err(AppError::youtube_api(
                "YouTube returned an oversized response.",
                true,
            ));
        }
    }
    let mut bytes = Vec::new();
    reader
        .take(MAX_RESPONSE_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| AppError::network("The YouTube response was incomplete."))?;
    if bytes.len() as u64 > MAX_RESPONSE_BYTES {
        return Err(AppError::youtube_api(
            "YouTube returned an oversized response.",
            true,
        ));
    }
    if !status.is_success() {
        return Err(match status {
            StatusCode::UNAUTHORIZED => {
                AppError::auth_required("The YouTube session expired. Reconnect the channel.")
            }
            StatusCode::TOO_MANY_REQUESTS => {
                AppError::youtube_api("YouTube rate-limited the request. Retry later.", true)
            }
            status if status.is_server_error() => {
                AppError::youtube_api("YouTube is temporarily unavailable. Retry later.", true)
            }
            _ => AppError::youtube_api(google_error_safe_detail(&bytes, safe_detail, api), false),
        });
    }
    serde_json::from_slice(&bytes)
        .map_err(|_| AppError::youtube_api("YouTube returned an invalid response.", true))
}

fn google_error_safe_detail(bytes: &[u8], fallback: &str, api: GoogleApiContext) -> String {
    let Ok(body) = serde_json::from_slice::<serde_json::Value>(bytes) else {
        return fallback.to_owned();
    };
    let reason = body
        .pointer("/error/errors/0/reason")
        .and_then(serde_json::Value::as_str)
        .or_else(|| body.get("error").and_then(serde_json::Value::as_str));
    match reason {
        Some("accessNotConfigured" | "serviceDisabled") => match api {
            GoogleApiContext::YouTubeData => {
                "Enable the YouTube Data API v3 for this OAuth client's Google Cloud project, then retry."
                    .to_owned()
            }
            GoogleApiContext::YouTubeAnalytics => {
                "Enable the YouTube Analytics API for this OAuth client's Google Cloud project, then retry."
                    .to_owned()
            }
            GoogleApiContext::OAuth => fallback.to_owned(),
        },
        Some("youtubeSignupRequired") => {
            "The authorized Google account has no YouTube channel. Reconnect with the account that owns the channel."
                .to_owned()
        }
        Some("quotaExceeded" | "dailyLimitExceeded") => {
            "The YouTube API quota is exhausted. Retry after the quota resets.".to_owned()
        }
        Some("insufficientPermissions" | "forbidden") => {
            "The authorized account cannot access this YouTube channel. Reconnect with the channel owner account."
                .to_owned()
        }
        Some("invalid_client") => {
            "Google rejected this OAuth desktop client. Recreate the client credential and retry."
                .to_owned()
        }
        Some("invalid_grant") => {
            "Google rejected the expired or already-used authorization. Start a fresh connection."
                .to_owned()
        }
        _ => fallback.to_owned(),
    }
}

fn parse_video_id(value: &str) -> Result<String, AppError> {
    let trimmed = value.trim();
    if is_video_id(trimmed) {
        return Ok(trimmed.to_owned());
    }
    let url = Url::parse(trimmed)
        .map_err(|_| AppError::invalid_argument("Enter a YouTube video URL or video ID."))?;
    let host = url.host_str().unwrap_or_default().to_ascii_lowercase();
    let candidate: Option<String> = if matches!(host.as_str(), "youtu.be" | "www.youtu.be") {
        url.path_segments()
            .and_then(|mut segments| segments.next())
            .map(str::to_owned)
    } else if matches!(
        host.as_str(),
        "youtube.com" | "www.youtube.com" | "m.youtube.com"
    ) {
        if url.path() == "/watch" {
            url.query_pairs()
                .find(|(key, _)| key == "v")
                .map(|(_, value)| value.into_owned())
        } else {
            let mut segments = url.path_segments().into_iter().flatten();
            match (segments.next(), segments.next()) {
                (Some("shorts" | "live" | "embed"), Some(video_id)) => Some(video_id.to_owned()),
                _ => None,
            }
        }
    } else {
        None
    };
    candidate.filter(|id| is_video_id(id)).ok_or_else(|| {
        AppError::invalid_argument("Enter a valid youtube.com or youtu.be video URL.")
    })
}

fn is_video_id(value: &str) -> bool {
    (6..=20).contains(&value.len())
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-'))
}

fn write_atomic(path: &Path, bytes: &[u8]) -> Result<(), AppError> {
    let temporary = path.with_extension("json.tmp");
    let backup = path.with_extension("json.bak");
    let mut file = File::create(&temporary).map_err(|_| {
        AppError::io(
            "YouTube performance data could not be saved.",
            "Check app-data permissions and retry.",
        )
    })?;
    file.write_all(bytes)
        .and_then(|()| file.sync_all())
        .map_err(|_| {
            AppError::io(
                "YouTube performance data could not be saved.",
                "Check available disk space and retry.",
            )
        })?;
    if path.exists() {
        let _ = fs::remove_file(&backup);
        fs::rename(path, &backup).map_err(|_| {
            AppError::io(
                "YouTube performance data could not be replaced.",
                "Close other copies of the app and retry.",
            )
        })?;
    }
    if fs::rename(&temporary, path).is_err() {
        if backup.exists() {
            let _ = fs::rename(&backup, path);
        }
        return Err(AppError::io(
            "YouTube performance data could not be published.",
            "The previous local data was preserved. Retry the operation.",
        ));
    }
    let _ = fs::remove_file(backup);
    Ok(())
}

fn now_rfc3339() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true)
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        io::{self, Cursor, Read},
    };

    use super::{
        analytics_daily, analytics_totals, build_authorization_url, decode_google_body,
        google_error_safe_detail, parse_oauth_callback_target, parse_video_id,
        read_http_request_head, video_candidate_for_channel, AnalyticsResponse, GoogleApiContext,
        VideoResource, YouTubeCatalog, YouTubePerformanceService, MAX_RESPONSE_BYTES,
    };
    use crate::domain::YouTubeConnectionPhase;

    #[test]
    fn authorization_url_uses_pkce_and_read_only_scopes() {
        let url = build_authorization_url(
            "client-id",
            "http://127.0.0.1:4242/oauth/callback",
            "state",
            "challenge",
        )
        .unwrap();
        let query: std::collections::HashMap<_, _> = url.query_pairs().into_owned().collect();
        assert_eq!(query["response_type"], "code");
        assert_eq!(query["code_challenge_method"], "S256");
        assert_eq!(query["code_challenge"], "challenge");
        assert!(query["scope"].contains("youtube.readonly"));
        assert!(query["scope"].contains("yt-analytics.readonly"));
        assert_eq!(query["access_type"], "offline");
    }

    #[test]
    fn video_id_parser_accepts_supported_youtube_urls_only() {
        for value in [
            "dQw4w9WgXcQ",
            "https://youtu.be/dQw4w9WgXcQ",
            "https://www.youtube.com/watch?v=dQw4w9WgXcQ",
            "https://youtube.com/shorts/dQw4w9WgXcQ",
            "https://m.youtube.com/live/dQw4w9WgXcQ",
        ] {
            assert_eq!(parse_video_id(value).unwrap(), "dQw4w9WgXcQ");
        }
        assert!(parse_video_id("https://example.com/watch?v=dQw4w9WgXcQ").is_err());
        assert!(parse_video_id("not a video").is_err());
    }

    #[test]
    fn callback_reader_stops_after_http_headers_and_validates_state() {
        let request =
            b"GET /oauth/callback?code=approved&state=expected HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n";
        let head = read_http_request_head(&mut Cursor::new(request)).unwrap();
        let target = head
            .lines()
            .next()
            .unwrap()
            .split_whitespace()
            .nth(1)
            .unwrap();
        assert_eq!(
            parse_oauth_callback_target(target, "expected").unwrap(),
            Some("approved".to_owned())
        );
        assert!(parse_oauth_callback_target(target, "wrong").is_err());
        assert!(parse_oauth_callback_target(
            "/oauth/callback?error=access_denied&state=expected",
            "expected",
        )
        .is_err());
    }

    #[test]
    fn callback_reader_ignores_an_empty_browser_probe() {
        struct EmptyTimeoutReader;

        impl Read for EmptyTimeoutReader {
            fn read(&mut self, _buffer: &mut [u8]) -> io::Result<usize> {
                Err(io::Error::new(
                    io::ErrorKind::TimedOut,
                    "browser probe sent no request",
                ))
            }
        }

        assert_eq!(read_http_request_head(&mut EmptyTimeoutReader).unwrap(), "");
    }

    #[test]
    fn connection_status_exposes_transient_browser_phase() {
        let root = std::env::temp_dir().join(format!("skcf-youtube-{}", uuid::Uuid::new_v4()));
        let service = YouTubePerformanceService::new(root);

        service
            .set_connection_phase(YouTubeConnectionPhase::AwaitingBrowser)
            .unwrap();

        assert_eq!(
            service.connection_status().unwrap().connection_phase,
            YouTubeConnectionPhase::AwaitingBrowser
        );
    }

    #[test]
    fn google_error_detail_maps_disabled_api_without_exposing_response() {
        let body = br#"{
            "error": {
                "message": "YouTube Data API v3 has not been used in project 123456.",
                "errors": [{"reason": "accessNotConfigured"}]
            }
        }"#;

        let data_detail = google_error_safe_detail(body, "fallback", GoogleApiContext::YouTubeData);
        let analytics_detail =
            google_error_safe_detail(body, "fallback", GoogleApiContext::YouTubeAnalytics);

        assert_eq!(
            data_detail,
            "Enable the YouTube Data API v3 for this OAuth client's Google Cloud project, then retry."
        );
        assert_eq!(
            analytics_detail,
            "Enable the YouTube Analytics API for this OAuth client's Google Cloud project, then retry."
        );
        assert!(!data_detail.contains("123456"));
        assert!(!analytics_detail.contains("123456"));
    }

    #[test]
    fn analytics_rows_map_by_header_name() {
        let response: AnalyticsResponse = serde_json::from_value(serde_json::json!({
            "columnHeaders": [
                {"name": "day"},
                {"name": "views"},
                {"name": "engagedViews"},
                {"name": "estimatedMinutesWatched"},
                {"name": "averageViewDuration"},
                {"name": "averageViewPercentage"},
                {"name": "likes"},
                {"name": "comments"},
                {"name": "shares"},
                {"name": "subscribersGained"},
                {"name": "subscribersLost"}
            ],
            "rows": [["2026-07-28", 20, 12, 8.5, 25.0, 45.2, 4, 2, 1, 3, 1]]
        }))
        .unwrap();
        let daily = analytics_daily(&response).unwrap();
        assert_eq!(daily[0].date, "2026-07-28");
        assert_eq!(daily[0].metrics.engaged_views, 12.0);
        assert_eq!(daily[0].metrics.views, 20.0);

        let aggregate: AnalyticsResponse = serde_json::from_value(serde_json::json!({
            "columnHeaders": response.column_headers.iter().skip(1).map(|header| {
                serde_json::json!({"name": header.name})
            }).collect::<Vec<_>>(),
            "rows": [[20, 12, 8.5, 25.0, 45.2, 4, 2, 1, 3, 1]]
        }))
        .unwrap();
        assert_eq!(
            analytics_totals(&aggregate).unwrap().subscribers_gained,
            3.0
        );
    }

    #[test]
    fn response_reader_rejects_oversized_bodies() {
        let body = vec![b'x'; MAX_RESPONSE_BYTES as usize + 1];
        assert!(decode_google_body::<serde_json::Value, _>(
            reqwest::StatusCode::OK,
            None,
            Cursor::new(body),
            "failed",
            GoogleApiContext::YouTubeData,
        )
        .is_err());
    }

    #[test]
    fn video_link_requires_the_connected_channel() {
        let video: VideoResource = serde_json::from_value(serde_json::json!({
            "id": "dQw4w9WgXcQ",
            "snippet": {
                "title": "Boss fight",
                "publishedAt": "2026-07-28T12:00:00Z",
                "channelId": "expected-channel"
            }
        }))
        .unwrap();
        assert!(video_candidate_for_channel(video, "other-channel").is_err());
    }

    #[test]
    fn local_catalog_is_versioned_and_atomic() {
        let root = std::env::temp_dir().join(format!("skcf-youtube-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&root).unwrap();
        let service = YouTubePerformanceService::new(root.clone());
        let catalog = YouTubeCatalog::default();

        service.save_catalog(&catalog).unwrap();
        assert_eq!(service.load_catalog().unwrap().version, 1);
        assert!(!service.catalog_path.with_extension("json.tmp").exists());
        assert!(!service.catalog_path.with_extension("json.bak").exists());

        fs::remove_dir_all(root).unwrap();
    }
}
