use std::fmt;
use std::process::Command;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayerState {
    Playing,
    Paused,
    Stopped,
    NotRunning,
    Unknown,
}

impl fmt::Display for PlayerState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Playing => write!(f, "Playing"),
            Self::Paused => write!(f, "Paused"),
            Self::Stopped => write!(f, "Stopped"),
            Self::NotRunning => write!(f, "Not Running"),
            Self::Unknown => write!(f, "Unknown"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrackInfo {
    pub state: PlayerState,
    pub uri: String,
    pub title: String,
    pub artist: String,
    pub track_number: u32,
    pub volume: u32,
}

#[derive(Debug)]
pub enum PlatformError {
    Execution(String),
    OutputDecode(String),
}

impl fmt::Display for PlatformError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Execution(err) => write!(f, "AppleScript execution error: {err}"),
            Self::OutputDecode(err) => write!(f, "Failed to decode AppleScript output: {err}"),
        }
    }
}

impl std::error::Error for PlatformError {}

const SPOTIFY_STATUS_SCRIPT: &str = r#"if application "Spotify" is running then
    tell application "Spotify"
        try
            set pState to player state as string
            if pState is "stopped" then
                return "stopped" & tab & tab & tab & tab & "0" & tab & "0"
            end if
            set sUrl to spotify url of current track
            set sName to name of current track
            set sArtist to artist of current track
            set sNum to track number of current track
            set sVol to sound volume
            return pState & tab & sUrl & tab & sName & tab & sArtist & tab & sNum & tab & sVol
        on error
            return "unknown" & tab & tab & tab & tab & "0" & tab & "0"
        end try
    end tell
else
    return "not_running" & tab & tab & tab & tab & "0" & tab & "0"
end if"#;

pub fn query_spotify() -> Result<TrackInfo, PlatformError> {
    let output = Command::new("osascript")
        .arg("-e")
        .arg(SPOTIFY_STATUS_SCRIPT)
        .output()
        .map_err(|err| PlatformError::Execution(err.to_string()))?;

    if !output.status.success() {
        let err_msg = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(PlatformError::Execution(err_msg));
    }

    let stdout = String::from_utf8(output.stdout)
        .map_err(|err| PlatformError::OutputDecode(err.to_string()))?;

    Ok(parse_status_output(stdout.trim()))
}

pub fn set_spotify_volume(volume: u32) -> Result<(), PlatformError> {
    let script = format!(
        "if application \"Spotify\" is running then tell application \"Spotify\" to set sound volume to {volume}"
    );
    let output = Command::new("osascript")
        .arg("-e")
        .arg(&script)
        .output()
        .map_err(|err| PlatformError::Execution(err.to_string()))?;

    if !output.status.success() {
        let err_msg = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(PlatformError::Execution(err_msg));
    }

    Ok(())
}

pub fn set_system_muted(muted: bool) -> Result<(), PlatformError> {
    let script = if muted {
        "set volume with output muted"
    } else {
        "set volume without output muted"
    };
    let output = Command::new("osascript")
        .arg("-e")
        .arg(script)
        .output()
        .map_err(|err| PlatformError::Execution(err.to_string()))?;

    if !output.status.success() {
        let err_msg = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(PlatformError::Execution(err_msg));
    }

    Ok(())
}

pub fn parse_status_output(raw: &str) -> TrackInfo {
    let parts: Vec<&str> = raw.split('\t').collect();
    let state_str = parts.first().copied().unwrap_or("unknown");
    let state = match state_str.to_lowercase().as_str() {
        "playing" => PlayerState::Playing,
        "paused" => PlayerState::Paused,
        "stopped" => PlayerState::Stopped,
        "not_running" => PlayerState::NotRunning,
        _ => PlayerState::Unknown,
    };

    let uri = parts.get(1).copied().unwrap_or("").trim().to_string();
    let title = parts.get(2).copied().unwrap_or("").trim().to_string();
    let artist = parts.get(3).copied().unwrap_or("").trim().to_string();
    let track_number = parts
        .get(4)
        .copied()
        .unwrap_or("0")
        .trim()
        .parse::<u32>()
        .unwrap_or(0);
    let volume = parts
        .get(5)
        .copied()
        .unwrap_or("0")
        .trim()
        .parse::<u32>()
        .unwrap_or(0);

    TrackInfo {
        state,
        uri,
        title,
        artist,
        track_number,
        volume,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_playing_track() {
        let raw = "playing\tspotify:track:123\tSong Name\tArtist Name\t5\t80";
        let info = parse_status_output(raw);
        assert_eq!(info.state, PlayerState::Playing);
        assert_eq!(info.uri, "spotify:track:123");
        assert_eq!(info.title, "Song Name");
        assert_eq!(info.artist, "Artist Name");
        assert_eq!(info.track_number, 5);
        assert_eq!(info.volume, 80);
    }

    #[test]
    fn parses_not_running() {
        let raw = "not_running\t\t\t\t0\t0";
        let info = parse_status_output(raw);
        assert_eq!(info.state, PlayerState::NotRunning);
        assert!(info.uri.is_empty());
    }

    #[test]
    fn parses_malformed_input_gracefully() {
        let raw = "unexpected";
        let info = parse_status_output(raw);
        assert_eq!(info.state, PlayerState::Unknown);
        assert_eq!(info.track_number, 0);
        assert_eq!(info.volume, 0);
    }
}
