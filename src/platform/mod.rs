pub mod macos;

pub use macos::{
    query_spotify, set_spotify_volume, set_system_muted, PlatformError, PlayerState, TrackInfo,
};
