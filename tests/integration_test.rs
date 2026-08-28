use std::env;
use std::time::Duration;
use wuspotify_muter::{
    classify_track, ActionManager, AdAction, AdReason, Config, ConfigError, DetectionMethod,
    MediaClassification, PlayerState, TrackInfo,
};

#[test]
fn test_config_parsing_valid_combinations() {
    let args = vec![
        "wuspotify-muter".to_string(),
        "-m".to_string(),
        "applescript".to_string(),
        "-a".to_string(),
        "spotify-mute".to_string(),
        "-i".to_string(),
        "250".to_string(),
    ];
    let config = Config::parse_from_args(args).expect("valid args");
    assert_eq!(config.method, DetectionMethod::AppleScript);
    assert_eq!(config.action, AdAction::SpotifyMute);
    assert_eq!(config.poll_interval, Duration::from_millis(250));
    assert!(config.verbose);

    let system_mute_args = vec![
        "wuspotify-muter".to_string(),
        "-a".to_string(),
        "mute".to_string(),
    ];
    let system_config = Config::parse_from_args(system_mute_args).expect("valid mute action");
    assert_eq!(system_config.action, AdAction::Mute);

    let quiet_args = vec![
        "wuspotify-muter".to_string(),
        "-q".to_string(),
        "--no-mute".to_string(),
    ];
    let quiet_config = Config::parse_from_args(quiet_args).expect("valid quiet args");
    assert_eq!(quiet_config.action, AdAction::None);
    assert!(!quiet_config.verbose);
}

#[test]
fn test_config_parsing_invalid_flags() {
    let args = vec!["wuspotify-muter".to_string(), "--invalid-flag".to_string()];
    let err = Config::parse_from_args(args).expect_err("unknown flag");
    assert!(matches!(err, ConfigError::UnknownOption(_)));
}

#[test]
fn test_config_parsing_invalid_action() {
    let args = vec![
        "wuspotify-muter".to_string(),
        "--action".to_string(),
        "unsupported".to_string(),
    ];
    let err = Config::parse_from_args(args).expect_err("invalid action");
    assert!(matches!(err, ConfigError::InvalidAction(_)));
}

#[test]
fn test_detection_heuristics_matrix() {
    let regular_track = TrackInfo {
        state: PlayerState::Playing,
        uri: "spotify:track:4cOdK2wGLETKBW3PvgPWqT".to_string(),
        title: "Never Gonna Give You Up".to_string(),
        artist: "Rick Astley".to_string(),
        track_number: 1,
        volume: 100,
    };
    assert_eq!(classify_track(&regular_track), MediaClassification::Music);

    let ad_track = TrackInfo {
        state: PlayerState::Playing,
        uri: "spotify:ad:123456789".to_string(),
        title: "Advertisement".to_string(),
        artist: "Spotify".to_string(),
        track_number: 0,
        volume: 100,
    };
    assert_eq!(
        classify_track(&ad_track),
        MediaClassification::Ad(AdReason::ExplicitAdUri)
    );
}

#[test]
fn test_action_manager_noop_mode() {
    let state_file = env::temp_dir().join("test_wuspotify_integration_noop.state");
    let mut mgr = ActionManager::with_state_file(AdAction::None, state_file.clone());

    let track = TrackInfo {
        state: PlayerState::Playing,
        uri: "spotify:ad:999".to_string(),
        title: "Ad".to_string(),
        artist: "Spotify".to_string(),
        track_number: 0,
        volume: 80,
    };

    assert_eq!(mgr.on_ad_start(&track).unwrap(), None);
    assert!(!mgr.is_muted());
    assert!(!state_file.exists());
}
