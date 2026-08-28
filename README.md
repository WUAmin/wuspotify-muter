# wuspotify-muter

Lightweight macOS CLI tool written in Rust that monitors the Spotify desktop app and automatically mutes audio ads.

## Prerequisites

- macOS 11+ (Big Sur, Monterey, Ventura, Sonoma, Sequoia — Apple Silicon & Intel)
- Spotify desktop application
- Rust (optional, only needed if building from source)

## Install / Download

### Pre-built Binaries (Recommended)
Download the latest release tarball for your Mac:
- **Universal** (works on any Mac): `wuspotify-muter-*-macos-universal.tar.gz`
- **Apple Silicon (M-series)**: `wuspotify-muter-*-macos-arm64.tar.gz`
- **Intel**: `wuspotify-muter-*-macos-x86_64.tar.gz`

### Build from Source

```bash
cargo build --release
```

The binary will be generated at `./target/release/wuspotify-muter`.

## How to Use

Run the binary while Spotify is running:

```bash
cargo run --release
```

Expected output (verbose by default with `spotify-mute`):
```text
[14:45:00] [INFO] Starting wuspotify-muter (method: applescript, action: spotify-mute, interval: 1000ms)...
[14:45:01] [MUSIC] Playing: Artist - Song Title (spotify:track:...)
[14:46:12] [VOLUME] Spotify sound volume: 75% -> 85%
[14:48:30] [AD DETECTED] Title: 'Advertisement' | Artist: 'Spotify' | URI: 'spotify:ad:...' | Reason: URI matches Spotify ad scheme
[14:48:30] [ACTION: MUTED] Spotify sound volume: 85% -> 0%
[14:49:00] [ACTION: UNMUTED] Restored Spotify sound volume to 85%
[14:49:00] [MUSIC] Playing: Artist - Next Song Title (spotify:track:...)
```

To mute macOS system/OS/global audio instead of Spotify in-app volume:
```bash
cargo run --release -- --action mute
```

To run in **quiet mode** (only print ad detections and muting actions):
```bash
cargo run --release -- --quiet
```

Press `Ctrl+C` at any time to exit gracefully (restoring previous volume if stopped mid-ad).

## Config & CLI Switches

| Option | Flag | Description | Default |
| :--- | :--- | :--- | :--- |
| Action | `-a, --action <ACTION>` | Action on ad detection (`spotify-mute`, `mute` / system audio, `none` / `log-only`) | `spotify-mute` |
| Method | `-m, --method <METHOD>` | Detection method to use (`applescript`) | `applescript` |
| Interval | `-i, --poll-interval-ms <MS>` | Polling frequency in milliseconds (min `100`) | `1000` |
| Quiet | `-q, --quiet`, `--no-verbose` | Quiet mode: suppress track updates and volume change logs | `false` |
| Verbose | `-v, --verbose` | Log music/podcast tracks, state changes, and volume changes | `true` |
| No-Mute | `--no-mute`, `--detect-only` | Detect and log ads only without modifying volume | `false` |
| Help | `-h, --help` | Display help message | |
| Version | `-V, --version` | Display version information | |

Example with custom settings:
```bash
wuspotify-muter --action mute --poll-interval-ms 500 --quiet
```

## Help

```bash
wuspotify-muter --help
```

## Limits

- **macOS only**: Uses macOS AppleScript interface to communicate with Spotify and macOS audio settings.
- **Desktop client only**: Does not monitor the Spotify web player or mobile devices.
- **HDMI / Digital Audio**: macOS does not support OS-level volume or mute controls for digital HDMI/DisplayPort output. Use `--action spotify-mute` (the default) for HDMI or Multi-Output setups.
- **Crash Recovery**: Saves current volume state to `$TMPDIR/wuspotify-muter.state` while muted so abrupt terminations automatically recover volume upon relaunch.

## Note

This project and its codebase are AI-generated.
