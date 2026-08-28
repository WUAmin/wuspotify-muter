use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

use crate::platform::{set_spotify_volume, set_system_muted, PlatformError, TrackInfo};

const STATE_FILENAME: &str = "wuspotify-muter.state";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdAction {
    SpotifyMute,
    Mute,
    None,
}

impl fmt::Display for AdAction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SpotifyMute => write!(f, "spotify-mute"),
            Self::Mute => write!(f, "mute"),
            Self::None => write!(f, "none"),
        }
    }
}

impl AdAction {
    pub fn parse(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "spotify-mute" | "spotifymute" | "spotify_mute" => Some(Self::SpotifyMute),
            "mute" | "system-mute" | "system_mute" | "os-mute" | "global-mute" => Some(Self::Mute),
            "none" | "log-only" | "log_only" | "log" => Some(Self::None),
            _ => None,
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum ActionLogEvent {
    SpotifyMuted { previous_volume: u32 },
    SpotifyUnmuted { restored_volume: u32 },
    SystemMuted,
    SystemUnmuted,
    RecoveredSpotifyOnStartup { restored_volume: u32 },
    RecoveredSystemOnStartup,
    RestoredSpotifyOnShutdown { restored_volume: u32 },
    RestoredSystemOnShutdown,
}

#[derive(Debug, PartialEq, Eq)]
pub enum SavedState {
    SpotifyMuted(u32),
    SystemMuted,
}

#[derive(Debug)]
pub struct ActionManager {
    action: AdAction,
    state_file: PathBuf,
    saved_volume: Option<u32>,
    is_muted: bool,
}

impl ActionManager {
    pub fn new(action: AdAction) -> Self {
        let state_file = std::env::temp_dir().join(STATE_FILENAME);
        Self::with_state_file(action, state_file)
    }

    pub fn with_state_file(action: AdAction, state_file: PathBuf) -> Self {
        Self {
            action,
            state_file,
            saved_volume: None,
            is_muted: false,
        }
    }

    pub fn action(&self) -> AdAction {
        self.action
    }

    pub fn is_muted(&self) -> bool {
        self.is_muted
    }

    pub fn init(&mut self) -> Result<Option<ActionLogEvent>, PlatformError> {
        if self.action == AdAction::None {
            return Ok(None);
        }

        match read_state_file(&self.state_file) {
            Some(SavedState::SpotifyMuted(saved_vol)) => {
                set_spotify_volume(saved_vol)?;
                remove_state_file(&self.state_file);
                self.saved_volume = None;
                self.is_muted = false;
                Ok(Some(ActionLogEvent::RecoveredSpotifyOnStartup {
                    restored_volume: saved_vol,
                }))
            }
            Some(SavedState::SystemMuted) => {
                set_system_muted(false)?;
                remove_state_file(&self.state_file);
                self.is_muted = false;
                Ok(Some(ActionLogEvent::RecoveredSystemOnStartup))
            }
            None => Ok(None),
        }
    }

    pub fn on_ad_start(
        &mut self,
        track: &TrackInfo,
    ) -> Result<Option<ActionLogEvent>, PlatformError> {
        if self.is_muted {
            return Ok(None);
        }

        match self.action {
            AdAction::SpotifyMute => {
                let vol_to_save = if track.volume > 0 {
                    track.volume
                } else {
                    self.saved_volume.unwrap_or(100)
                };

                self.saved_volume = Some(vol_to_save);
                self.is_muted = true;
                write_state_file(&self.state_file, &format!("spotify:{vol_to_save}"));
                set_spotify_volume(0)?;

                Ok(Some(ActionLogEvent::SpotifyMuted {
                    previous_volume: vol_to_save,
                }))
            }
            AdAction::Mute => {
                self.is_muted = true;
                write_state_file(&self.state_file, "system");
                set_system_muted(true)?;

                Ok(Some(ActionLogEvent::SystemMuted))
            }
            AdAction::None => Ok(None),
        }
    }

    pub fn on_media_start(
        &mut self,
        _track: &TrackInfo,
    ) -> Result<Option<ActionLogEvent>, PlatformError> {
        if !self.is_muted {
            return Ok(None);
        }

        match self.action {
            AdAction::SpotifyMute => {
                let vol_to_restore = self.saved_volume.unwrap_or(100);
                set_spotify_volume(vol_to_restore)?;
                remove_state_file(&self.state_file);
                self.saved_volume = None;
                self.is_muted = false;

                Ok(Some(ActionLogEvent::SpotifyUnmuted {
                    restored_volume: vol_to_restore,
                }))
            }
            AdAction::Mute => {
                set_system_muted(false)?;
                remove_state_file(&self.state_file);
                self.is_muted = false;

                Ok(Some(ActionLogEvent::SystemUnmuted))
            }
            AdAction::None => Ok(None),
        }
    }

    pub fn on_shutdown(&mut self) -> Result<Option<ActionLogEvent>, PlatformError> {
        if !self.is_muted {
            return Ok(None);
        }

        match self.action {
            AdAction::SpotifyMute => {
                let vol_to_restore = self.saved_volume.unwrap_or(100);
                set_spotify_volume(vol_to_restore)?;
                remove_state_file(&self.state_file);
                self.saved_volume = None;
                self.is_muted = false;

                Ok(Some(ActionLogEvent::RestoredSpotifyOnShutdown {
                    restored_volume: vol_to_restore,
                }))
            }
            AdAction::Mute => {
                set_system_muted(false)?;
                remove_state_file(&self.state_file);
                self.is_muted = false;

                Ok(Some(ActionLogEvent::RestoredSystemOnShutdown))
            }
            AdAction::None => Ok(None),
        }
    }
}

fn write_state_file(path: &Path, content: &str) {
    let _ = fs::write(path, content);
}

fn read_state_file(path: &Path) -> Option<SavedState> {
    let content = fs::read_to_string(path).ok()?;
    let trimmed = content.trim();

    if trimmed == "system" {
        return Some(SavedState::SystemMuted);
    }

    if let Some(vol_str) = trimmed.strip_prefix("spotify:") {
        return vol_str.parse::<u32>().ok().map(SavedState::SpotifyMuted);
    }

    // Backwards compatibility for plain numeric volume
    trimmed.parse::<u32>().ok().map(SavedState::SpotifyMuted)
}

fn remove_state_file(path: &Path) {
    let _ = fs::remove_file(path);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::PlayerState;
    use std::env;

    fn test_track(volume: u32) -> TrackInfo {
        TrackInfo {
            state: PlayerState::Playing,
            uri: "spotify:track:123".to_string(),
            title: "Song".to_string(),
            artist: "Artist".to_string(),
            track_number: 1,
            volume,
        }
    }

    #[test]
    fn parses_action_strings() {
        assert_eq!(AdAction::parse("spotify-mute"), Some(AdAction::SpotifyMute));
        assert_eq!(AdAction::parse("SPOTIFY-MUTE"), Some(AdAction::SpotifyMute));
        assert_eq!(AdAction::parse("mute"), Some(AdAction::Mute));
        assert_eq!(AdAction::parse("system-mute"), Some(AdAction::Mute));
        assert_eq!(AdAction::parse("os-mute"), Some(AdAction::Mute));
        assert_eq!(AdAction::parse("global-mute"), Some(AdAction::Mute));
        assert_eq!(AdAction::parse("none"), Some(AdAction::None));
        assert_eq!(AdAction::parse("log-only"), Some(AdAction::None));
        assert_eq!(AdAction::parse("log_only"), Some(AdAction::None));
        assert_eq!(AdAction::parse("unknown"), None);
    }

    #[test]
    fn state_file_io_helpers() {
        let temp_dir = env::temp_dir();
        let state_path = temp_dir.join("test_wuspotify_state_io.state");

        write_state_file(&state_path, "spotify:75");
        assert_eq!(
            read_state_file(&state_path),
            Some(SavedState::SpotifyMuted(75))
        );

        write_state_file(&state_path, "system");
        assert_eq!(read_state_file(&state_path), Some(SavedState::SystemMuted));

        remove_state_file(&state_path);
        assert_eq!(read_state_file(&state_path), None);
    }

    #[test]
    fn none_action_does_not_mutate_state() {
        let temp_dir = env::temp_dir();
        let state_path = temp_dir.join("test_wuspotify_none.state");
        let mut manager = ActionManager::with_state_file(AdAction::None, state_path.clone());

        let track = test_track(80);
        let event = manager.on_ad_start(&track).unwrap();
        assert_eq!(event, None);
        assert!(!manager.is_muted());
        assert_eq!(read_state_file(&state_path), None);
    }
}
