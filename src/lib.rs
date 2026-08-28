pub mod action;
pub mod detector;
pub mod platform;

use std::fmt;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, SystemTime};

pub use action::{ActionLogEvent, ActionManager, AdAction};
pub use detector::{classify_track, AdReason, DetectionEvent, DetectorState, MediaClassification};
pub use platform::{
    query_spotify, set_spotify_volume, set_system_muted, PlatformError, PlayerState, TrackInfo,
};

pub const DEFAULT_POLL_INTERVAL_MS: u64 = 1000;
pub const MIN_POLL_INTERVAL_MS: u64 = 100;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DetectionMethod {
    AppleScript,
}

impl fmt::Display for DetectionMethod {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::AppleScript => write!(f, "applescript"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Config {
    pub method: DetectionMethod,
    pub action: AdAction,
    pub poll_interval: Duration,
    pub verbose: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            method: DetectionMethod::AppleScript,
            action: AdAction::SpotifyMute,
            poll_interval: Duration::from_millis(DEFAULT_POLL_INTERVAL_MS),
            verbose: true,
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum ConfigError {
    HelpRequested,
    VersionRequested,
    UnknownOption(String),
    MissingValue(String),
    InvalidMethod(String),
    InvalidAction(String),
    InvalidInterval(String),
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::HelpRequested => write!(f, "{}", help_message()),
            Self::VersionRequested => write!(f, "wuspotify-muter {}", env!("CARGO_PKG_VERSION")),
            Self::UnknownOption(opt) => write!(f, "Unknown option '{opt}'. Use --help for usage."),
            Self::MissingValue(opt) => write!(f, "Option '{opt}' requires a value."),
            Self::InvalidMethod(val) => write!(
                f,
                "Unsupported detection method '{val}'. Supported methods: 'applescript'. (Planned: 'notification', 'web-api', 'window-title')"
            ),
            Self::InvalidAction(val) => write!(
                f,
                "Unsupported action '{val}'. Supported actions: 'spotify-mute', 'mute' (system audio), 'none' (or 'log-only'). (Planned: 'skip', 'restart')"
            ),
            Self::InvalidInterval(val) => write!(
                f,
                "Invalid poll interval '{val}': must be an integer >= {MIN_POLL_INTERVAL_MS} ms."
            ),
        }
    }
}

impl std::error::Error for ConfigError {}

pub fn help_message() -> String {
    format!(
        "wuspotify-muter {}\n\
        A lightweight macOS CLI to detect and mute Spotify advertisements.\n\n\
        USAGE:\n\
            wuspotify-muter [OPTIONS]\n\n\
        OPTIONS:\n\
            -a, --action <ACTION>            Action to take on ad detection (default: spotify-mute)\n\
                                             Supported: spotify-mute, mute (system audio), none (or log-only)\n\
            -m, --method <METHOD>            Detection method to use (default: applescript)\n\
                                             Supported: applescript\n\
            -i, --poll-interval-ms <MS>      Polling interval in milliseconds (default: {DEFAULT_POLL_INTERVAL_MS})\n\
            -q, --quiet, --no-verbose        Quiet mode: only log ads and muting actions (suppress music tracks and volume changes)\n\
            -v, --verbose                    Verbose mode (default: enabled)\n\
                --no-mute, --detect-only     Shortcut for --action none\n\
            -h, --help                       Print help information\n\
            -V, --version                    Print version information\n",
        env!("CARGO_PKG_VERSION")
    )
}

impl Config {
    pub fn parse_from_args<I>(args: I) -> Result<Self, ConfigError>
    where
        I: IntoIterator<Item = String>,
    {
        let mut config = Self::default();
        let mut iter = args.into_iter().skip(1);

        while let Some(arg) = iter.next() {
            match arg.as_str() {
                "-h" | "--help" => return Err(ConfigError::HelpRequested),
                "-V" | "--version" => return Err(ConfigError::VersionRequested),
                "-v" | "--verbose" => config.verbose = true,
                "-q" | "--quiet" | "--no-verbose" => config.verbose = false,
                "--no-mute" | "--detect-only" => config.action = AdAction::None,
                "-a" | "--action" => {
                    let value = iter
                        .next()
                        .ok_or_else(|| ConfigError::MissingValue(arg.clone()))?;
                    match AdAction::parse(&value) {
                        Some(act) => config.action = act,
                        None => return Err(ConfigError::InvalidAction(value)),
                    }
                }
                "-m" | "--method" => {
                    let value = iter
                        .next()
                        .ok_or_else(|| ConfigError::MissingValue(arg.clone()))?;
                    match value.to_lowercase().as_str() {
                        "applescript" => config.method = DetectionMethod::AppleScript,
                        _ => return Err(ConfigError::InvalidMethod(value)),
                    }
                }
                "-i" | "--poll-interval-ms" => {
                    let value = iter
                        .next()
                        .ok_or_else(|| ConfigError::MissingValue(arg.clone()))?;
                    let ms = value
                        .parse::<u64>()
                        .map_err(|_| ConfigError::InvalidInterval(value.clone()))?;
                    if ms < MIN_POLL_INTERVAL_MS {
                        return Err(ConfigError::InvalidInterval(value));
                    }
                    config.poll_interval = Duration::from_millis(ms);
                }
                _ => return Err(ConfigError::UnknownOption(arg)),
            }
        }

        Ok(config)
    }
}

#[derive(Debug)]
pub enum AppError {
    Platform(PlatformError),
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Platform(err) => write!(f, "Platform error: {err}"),
        }
    }
}

impl std::error::Error for AppError {}

impl From<PlatformError> for AppError {
    fn from(err: PlatformError) -> Self {
        Self::Platform(err)
    }
}

fn current_timestamp() -> String {
    let now = SystemTime::now();
    let duration = now
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default();
    let total_secs = duration.as_secs();
    let hours = (total_secs / 3600) % 24;
    let minutes = (total_secs / 60) % 60;
    let seconds = total_secs % 60;
    format!("{hours:02}:{minutes:02}:{seconds:02}")
}

pub fn run(config: Config, running: Arc<AtomicBool>) -> Result<(), AppError> {
    println!(
        "[{}] [INFO] Starting wuspotify-muter (method: {}, action: {}, interval: {}ms)...",
        current_timestamp(),
        config.method,
        config.action,
        config.poll_interval.as_millis()
    );

    let mut detector = DetectorState::new();
    let mut action_mgr = ActionManager::new(config.action);

    if let Some(log_event) = action_mgr.init()? {
        log_action_event(&log_event);
    }

    while running.load(Ordering::Relaxed) {
        let track_info = match config.method {
            DetectionMethod::AppleScript => query_spotify()?,
        };

        if let Some(event) = detector.process(&track_info) {
            match &event {
                DetectionEvent::AdStarted { .. } => {
                    handle_event(&event, config.verbose);
                    if let Some(log) = action_mgr.on_ad_start(&track_info)? {
                        log_action_event(&log);
                    }
                }
                DetectionEvent::MediaStarted { .. } => {
                    if let Some(log) = action_mgr.on_media_start(&track_info)? {
                        log_action_event(&log);
                    }
                    handle_event(&event, config.verbose);
                }
                DetectionEvent::StateChanged { .. } => {
                    handle_event(&event, config.verbose);
                }
                DetectionEvent::VolumeChanged { current, .. } => {
                    // Suppress duplicate volume log when muter actively sets Spotify volume to 0
                    if !action_mgr.is_muted() || *current != 0 {
                        handle_event(&event, config.verbose);
                    }
                }
            }
        }

        thread::sleep(config.poll_interval);
    }

    if let Some(log) = action_mgr.on_shutdown()? {
        log_action_event(&log);
    }

    println!(
        "[{}] [INFO] Stopping wuspotify-muter gracefully.",
        current_timestamp()
    );
    Ok(())
}

fn log_action_event(event: &ActionLogEvent) {
    let ts = current_timestamp();
    match event {
        ActionLogEvent::SpotifyMuted { previous_volume } => {
            println!("[{ts}] [ACTION: MUTED] Spotify sound volume: {previous_volume}% -> 0%");
        }
        ActionLogEvent::SpotifyUnmuted { restored_volume } => {
            println!(
                "[{ts}] [ACTION: UNMUTED] Restored Spotify sound volume to {restored_volume}%"
            );
        }
        ActionLogEvent::SystemMuted => {
            println!("[{ts}] [ACTION: MUTED] macOS system audio muted");
        }
        ActionLogEvent::SystemUnmuted => {
            println!("[{ts}] [ACTION: UNMUTED] Restored macOS system audio");
        }
        ActionLogEvent::RecoveredSpotifyOnStartup { restored_volume } => {
            println!(
                "[{ts}] [RECOVERY] Restored unrecovered pre-crash Spotify volume: {restored_volume}%"
            );
        }
        ActionLogEvent::RecoveredSystemOnStartup => {
            println!("[{ts}] [RECOVERY] Restored unrecovered pre-crash macOS system audio");
        }
        ActionLogEvent::RestoredSpotifyOnShutdown { restored_volume } => {
            println!(
                "[{ts}] [ACTION: RESTORED] Restored Spotify sound volume to {restored_volume}% on exit"
            );
        }
        ActionLogEvent::RestoredSystemOnShutdown => {
            println!("[{ts}] [ACTION: RESTORED] Restored macOS system audio on exit");
        }
    }
}

fn handle_event(event: &DetectionEvent, verbose: bool) {
    let ts = current_timestamp();
    match event {
        DetectionEvent::AdStarted {
            title,
            artist,
            uri,
            reason,
        } => {
            println!(
                "[{ts}] [AD DETECTED] Title: '{title}' | Artist: '{artist}' | URI: '{uri}' | Reason: {reason}"
            );
        }
        DetectionEvent::MediaStarted {
            title,
            artist,
            uri,
            is_podcast,
        } => {
            if verbose {
                let tag = if *is_podcast { "PODCAST" } else { "MUSIC" };
                println!("[{ts}] [{tag}] Playing: {artist} - {title} ({uri})");
            }
        }
        DetectionEvent::StateChanged { current, .. } => {
            if verbose {
                println!("[{ts}] [STATE] Spotify state: {current}");
            }
        }
        DetectionEvent::VolumeChanged { previous, current } => {
            if verbose {
                println!("[{ts}] [VOLUME] Spotify sound volume: {previous}% -> {current}%");
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_default_config() {
        let args = vec!["wuspotify-muter".to_string()];
        let cfg = Config::parse_from_args(args).expect("should parse default");
        assert_eq!(cfg.method, DetectionMethod::AppleScript);
        assert_eq!(cfg.action, AdAction::SpotifyMute);
        assert_eq!(cfg.poll_interval, Duration::from_millis(1000));
        assert!(cfg.verbose);
    }

    #[test]
    fn parses_flags_correctly() {
        let args = vec![
            "wuspotify-muter".to_string(),
            "--method".to_string(),
            "applescript".to_string(),
            "--action".to_string(),
            "mute".to_string(),
            "--poll-interval-ms".to_string(),
            "500".to_string(),
            "--quiet".to_string(),
        ];
        let cfg = Config::parse_from_args(args).expect("should parse flags");
        assert_eq!(cfg.method, DetectionMethod::AppleScript);
        assert_eq!(cfg.action, AdAction::Mute);
        assert_eq!(cfg.poll_interval, Duration::from_millis(500));
        assert!(!cfg.verbose);
    }

    #[test]
    fn parses_no_mute_shortcut() {
        let args = vec!["wuspotify-muter".to_string(), "--no-mute".to_string()];
        let cfg = Config::parse_from_args(args).expect("should parse no-mute");
        assert_eq!(cfg.action, AdAction::None);
    }

    #[test]
    fn rejects_unsupported_action() {
        let args = vec![
            "wuspotify-muter".to_string(),
            "--action".to_string(),
            "invalid_action".to_string(),
        ];
        let err = Config::parse_from_args(args).expect_err("should reject");
        assert!(matches!(err, ConfigError::InvalidAction(_)));
    }

    #[test]
    fn rejects_unsupported_method() {
        let args = vec![
            "wuspotify-muter".to_string(),
            "--method".to_string(),
            "window-title".to_string(),
        ];
        let err = Config::parse_from_args(args).expect_err("should reject");
        assert!(matches!(err, ConfigError::InvalidMethod(_)));
    }

    #[test]
    fn rejects_too_low_interval() {
        let args = vec![
            "wuspotify-muter".to_string(),
            "--poll-interval-ms".to_string(),
            "50".to_string(),
        ];
        let err = Config::parse_from_args(args).expect_err("should reject");
        assert!(matches!(err, ConfigError::InvalidInterval(_)));
    }
}
