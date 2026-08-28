use std::fmt;

use crate::platform::{PlayerState, TrackInfo};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdReason {
    ExplicitAdUri,
    NonStandardUri,
    EmptyUriWhilePlaying,
    ZeroTrackNumber,
    AdvertisementTitle,
}

impl fmt::Display for AdReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ExplicitAdUri => write!(f, "URI matches Spotify ad scheme"),
            Self::NonStandardUri => write!(f, "Non-track/podcast URI playing"),
            Self::EmptyUriWhilePlaying => write!(f, "Empty track URI during playback"),
            Self::ZeroTrackNumber => write!(f, "Track number is 0 with ad metadata"),
            Self::AdvertisementTitle => write!(f, "Track title is 'Advertisement'"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MediaClassification {
    Music,
    Podcast,
    Ad(AdReason),
    Inactive,
}

impl fmt::Display for MediaClassification {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Music => write!(f, "Music"),
            Self::Podcast => write!(f, "Podcast"),
            Self::Ad(reason) => write!(f, "Ad ({reason})"),
            Self::Inactive => write!(f, "Inactive"),
        }
    }
}

pub fn classify_track(info: &TrackInfo) -> MediaClassification {
    if info.state != PlayerState::Playing {
        return MediaClassification::Inactive;
    }

    if info.uri.starts_with("spotify:ad:") || info.uri.starts_with("spotify:advertisement:") {
        return MediaClassification::Ad(AdReason::ExplicitAdUri);
    }

    if info.title.eq_ignore_ascii_case("advertisement") {
        return MediaClassification::Ad(AdReason::AdvertisementTitle);
    }

    if info.uri.is_empty() {
        return MediaClassification::Ad(AdReason::EmptyUriWhilePlaying);
    }

    if info.uri.starts_with("spotify:track:") {
        if info.track_number == 0
            && (info.artist.is_empty() || info.artist.eq_ignore_ascii_case("spotify"))
        {
            return MediaClassification::Ad(AdReason::ZeroTrackNumber);
        }
        return MediaClassification::Music;
    }

    if info.uri.starts_with("spotify:episode:") {
        return MediaClassification::Podcast;
    }

    MediaClassification::Ad(AdReason::NonStandardUri)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DetectionEvent {
    AdStarted {
        title: String,
        artist: String,
        uri: String,
        reason: AdReason,
    },
    MediaStarted {
        title: String,
        artist: String,
        uri: String,
        is_podcast: bool,
    },
    StateChanged {
        previous: Option<PlayerState>,
        current: PlayerState,
    },
    VolumeChanged {
        previous: u32,
        current: u32,
    },
}

#[derive(Debug, Default)]
pub struct DetectorState {
    last_uri: Option<String>,
    last_state: Option<PlayerState>,
    last_classification: Option<MediaClassification>,
    last_volume: Option<u32>,
}

impl DetectorState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn process(&mut self, info: &TrackInfo) -> Option<DetectionEvent> {
        let classification = classify_track(info);
        let state_changed = self.last_state != Some(info.state);
        let uri_changed = self.last_uri.as_deref() != Some(&info.uri);
        let volume_changed = match self.last_volume {
            Some(prev) => {
                prev != info.volume
                    && info.state != PlayerState::NotRunning
                    && info.state != PlayerState::Stopped
            }
            None => false,
        };

        let event = if info.state == PlayerState::Playing {
            if uri_changed || self.last_classification != Some(classification.clone()) {
                match &classification {
                    MediaClassification::Ad(reason) => Some(DetectionEvent::AdStarted {
                        title: info.title.clone(),
                        artist: info.artist.clone(),
                        uri: info.uri.clone(),
                        reason: *reason,
                    }),
                    MediaClassification::Music => Some(DetectionEvent::MediaStarted {
                        title: info.title.clone(),
                        artist: info.artist.clone(),
                        uri: info.uri.clone(),
                        is_podcast: false,
                    }),
                    MediaClassification::Podcast => Some(DetectionEvent::MediaStarted {
                        title: info.title.clone(),
                        artist: info.artist.clone(),
                        uri: info.uri.clone(),
                        is_podcast: true,
                    }),
                    MediaClassification::Inactive => None,
                }
            } else if volume_changed {
                Some(DetectionEvent::VolumeChanged {
                    previous: self.last_volume.unwrap_or(info.volume),
                    current: info.volume,
                })
            } else {
                None
            }
        } else if state_changed {
            Some(DetectionEvent::StateChanged {
                previous: self.last_state,
                current: info.state,
            })
        } else if volume_changed {
            Some(DetectionEvent::VolumeChanged {
                previous: self.last_volume.unwrap_or(info.volume),
                current: info.volume,
            })
        } else {
            None
        };

        self.last_state = Some(info.state);
        self.last_uri = Some(info.uri.clone());
        self.last_classification = Some(classification);
        self.last_volume = Some(info.volume);

        event
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_track(
        state: PlayerState,
        uri: &str,
        title: &str,
        artist: &str,
        num: u32,
        volume: u32,
    ) -> TrackInfo {
        TrackInfo {
            state,
            uri: uri.to_string(),
            title: title.to_string(),
            artist: artist.to_string(),
            track_number: num,
            volume,
        }
    }

    #[test]
    fn classifies_regular_song_as_music() {
        let track = sample_track(
            PlayerState::Playing,
            "spotify:track:4cOdK2wGLETKBW3PvgPWqT",
            "Song",
            "Artist",
            1,
            50,
        );
        assert_eq!(classify_track(&track), MediaClassification::Music);
    }

    #[test]
    fn classifies_podcast_episode() {
        let track = sample_track(
            PlayerState::Playing,
            "spotify:episode:5k3v",
            "Episode Title",
            "Podcast Host",
            0,
            50,
        );
        assert_eq!(classify_track(&track), MediaClassification::Podcast);
    }

    #[test]
    fn classifies_explicit_ad_uri() {
        let track = sample_track(
            PlayerState::Playing,
            "spotify:ad:098234",
            "Sponsor",
            "Spotify",
            0,
            50,
        );
        assert_eq!(
            classify_track(&track),
            MediaClassification::Ad(AdReason::ExplicitAdUri)
        );
    }

    #[test]
    fn classifies_advertisement_title() {
        let track = sample_track(
            PlayerState::Playing,
            "spotify:track:unknown",
            "Advertisement",
            "Spotify",
            0,
            50,
        );
        assert_eq!(
            classify_track(&track),
            MediaClassification::Ad(AdReason::AdvertisementTitle)
        );
    }

    #[test]
    fn classifies_empty_uri_as_ad() {
        let track = sample_track(PlayerState::Playing, "", "Ad Title", "", 0, 50);
        assert_eq!(
            classify_track(&track),
            MediaClassification::Ad(AdReason::EmptyUriWhilePlaying)
        );
    }

    #[test]
    fn classifies_inactive_when_paused() {
        let track = sample_track(
            PlayerState::Paused,
            "spotify:track:123",
            "Song",
            "Artist",
            1,
            50,
        );
        assert_eq!(classify_track(&track), MediaClassification::Inactive);
    }

    #[test]
    fn detector_emits_event_on_ad_start() {
        let mut detector = DetectorState::new();

        let track1 = sample_track(
            PlayerState::Playing,
            "spotify:track:1",
            "Song A",
            "Artist A",
            1,
            50,
        );
        let event1 = detector.process(&track1);
        assert!(matches!(event1, Some(DetectionEvent::MediaStarted { .. })));

        let ad_track = sample_track(
            PlayerState::Playing,
            "spotify:ad:999",
            "Sponsor",
            "Spotify",
            0,
            50,
        );
        let event2 = detector.process(&ad_track);
        assert_eq!(
            event2,
            Some(DetectionEvent::AdStarted {
                title: "Sponsor".to_string(),
                artist: "Spotify".to_string(),
                uri: "spotify:ad:999".to_string(),
                reason: AdReason::ExplicitAdUri,
            })
        );
    }

    #[test]
    fn detector_emits_event_on_volume_change() {
        let mut detector = DetectorState::new();

        let track1 = sample_track(
            PlayerState::Playing,
            "spotify:track:1",
            "Song A",
            "Artist A",
            1,
            50,
        );
        let _ = detector.process(&track1);

        let track2 = sample_track(
            PlayerState::Playing,
            "spotify:track:1",
            "Song A",
            "Artist A",
            1,
            85,
        );
        let event2 = detector.process(&track2);
        assert_eq!(
            event2,
            Some(DetectionEvent::VolumeChanged {
                previous: 50,
                current: 85,
            })
        );
    }
}
