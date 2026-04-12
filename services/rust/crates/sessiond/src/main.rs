use std::env;
use std::fs;
use std::io::{BufRead, BufReader};
use std::os::unix::net::UnixStream;
use std::path::Path;
use std::thread;
use std::time::Duration;

use ipc_proto::{
    event_socket_path_from_env, parse_event_line, quiet_mode_flag_path_from_env, PlaybackEvent,
};

fn main() {
    if let Err(err) = run() {
        eprintln!("sessiond failed: {err}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let event_socket = event_socket_path_from_env();
    let quiet_flag = quiet_mode_flag_path_from_env();
    let protected_services = env::var("SESSIOND_PROTECTED_SERVICES").unwrap_or_default();
    let freezable_services = env::var("SESSIOND_FREEZABLE_SERVICES").unwrap_or_default();
    let mut quiet_state = QuietModeState::Off;

    println!("sessiond watching");
    println!("  event source:   {}", event_socket.display());
    println!("  quiet flag:     {}", quiet_flag.display());
    println!(
        "  protected svc:  {}",
        printable_service_list(&protected_services)
    );
    println!(
        "  freezable svc:  {}",
        printable_service_list(&freezable_services)
    );

    loop {
        match UnixStream::connect(&event_socket) {
            Ok(stream) => {
                if let Err(err) = handle_stream(stream, &quiet_flag, &mut quiet_state) {
                    eprintln!("sessiond stream error: {err}");
                }
            }
            Err(err) => eprintln!("sessiond connect retry: {err}"),
        }

        thread::sleep(Duration::from_millis(300));
    }
}

fn printable_service_list(raw: &str) -> String {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        "-".to_string()
    } else {
        trimmed.to_string()
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
enum QuietModeState {
    Off,
    Prepare,
    Active,
    ErrorHold,
}

impl QuietModeState {
    fn as_str(&self) -> &'static str {
        match self {
            Self::Off => "off",
            Self::Prepare => "prepare",
            Self::Active => "active",
            Self::ErrorHold => "error_hold",
        }
    }
}

fn handle_stream(
    stream: UnixStream,
    quiet_flag_path: &Path,
    quiet_state: &mut QuietModeState,
) -> Result<(), String> {
    let mut reader = BufReader::new(stream);

    loop {
        let mut line = String::new();
        let read = reader
            .read_line(&mut line)
            .map_err(|err| format!("read event: {err}"))?;
        if read == 0 {
            return Ok(());
        }

        let event = match parse_event_line(&line) {
            Ok(event) => event,
            Err(err) => {
                eprintln!("sessiond ignored malformed event: {err}");
                continue;
            }
        };

        *quiet_state = next_quiet_state(*quiet_state, &event);
        reconcile_quiet_flag(*quiet_state, quiet_flag_path)?;
    }
}

fn next_quiet_state(current: QuietModeState, event: &PlaybackEvent) -> QuietModeState {
    match event {
        PlaybackEvent::PlayRequestAccepted { .. } => QuietModeState::Prepare,
        PlaybackEvent::PlaybackStarted { .. } => QuietModeState::Active,
        PlaybackEvent::PlaybackResumed => QuietModeState::Active,
        PlaybackEvent::TrackChanged { .. } => match current {
            QuietModeState::Off => QuietModeState::Prepare,
            QuietModeState::Prepare | QuietModeState::Active | QuietModeState::ErrorHold => {
                QuietModeState::Active
            }
        },
        PlaybackEvent::PlaybackPaused | PlaybackEvent::PlaybackStopped { .. } => {
            QuietModeState::Off
        }
        PlaybackEvent::PlaybackFailed { keep_quiet, .. } => {
            if *keep_quiet {
                QuietModeState::ErrorHold
            } else {
                QuietModeState::Off
            }
        }
    }
}

fn reconcile_quiet_flag(state: QuietModeState, quiet_flag_path: &Path) -> Result<(), String> {
    match state {
        QuietModeState::Off => {
            if quiet_flag_path.exists() {
                fs::remove_file(quiet_flag_path).map_err(|err| {
                    format!("remove quiet flag {}: {err}", quiet_flag_path.display())
                })?;
            }
            Ok(())
        }
        QuietModeState::Prepare | QuietModeState::Active | QuietModeState::ErrorHold => {
            let parent = quiet_flag_path.parent().ok_or_else(|| {
                format!(
                    "quiet flag path has no parent directory: {}",
                    quiet_flag_path.display()
                )
            })?;
            fs::create_dir_all(parent)
                .map_err(|err| format!("create quiet flag dir {}: {err}", parent.display()))?;
            fs::write(quiet_flag_path, format!("{}\n", state.as_str()))
                .map_err(|err| format!("write quiet flag {}: {err}", quiet_flag_path.display()))?;
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{next_quiet_state, QuietModeState};
    use ipc_proto::{PlaybackEvent, PlaybackFailureClass};

    #[test]
    fn playback_start_transitions_to_active() {
        let state = next_quiet_state(
            QuietModeState::Prepare,
            &PlaybackEvent::PlaybackStarted {
                track_id: "demo".to_string(),
            },
        );

        assert_eq!(state, QuietModeState::Active);
    }

    #[test]
    fn recoverable_failure_keeps_quiet_mode() {
        let state = next_quiet_state(
            QuietModeState::Active,
            &PlaybackEvent::PlaybackFailed {
                reason: "file_unreadable".to_string(),
                class: PlaybackFailureClass::Content,
                recoverable: true,
                keep_quiet: true,
            },
        );

        assert_eq!(state, QuietModeState::ErrorHold);
    }

    #[test]
    fn stop_turns_quiet_mode_off() {
        let state = next_quiet_state(
            QuietModeState::Active,
            &PlaybackEvent::PlaybackStopped {
                reason: "user_stop".to_string(),
            },
        );

        assert_eq!(state, QuietModeState::Off);
    }
}
