use std::env;
use std::fs;
use std::io::{BufRead, BufReader};
use std::os::unix::net::UnixStream;
use std::path::Path;
use std::process::Command;
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
    let quiet_stop_units =
        parse_service_list(&env::var("SESSIOND_QUIET_STOP_UNITS").unwrap_or_default());
    let quiet_start_units =
        parse_service_list(&env::var("SESSIOND_QUIET_START_UNITS").unwrap_or_default());
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
    println!("  quiet stop:     {}", printable_units(&quiet_stop_units));
    println!("  quiet start:    {}", printable_units(&quiet_start_units));

    loop {
        match UnixStream::connect(&event_socket) {
            Ok(stream) => {
                if let Err(err) = handle_stream(
                    stream,
                    &quiet_flag,
                    &quiet_stop_units,
                    &quiet_start_units,
                    &mut quiet_state,
                ) {
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

fn parse_service_list(raw: &str) -> Vec<String> {
    raw.split_whitespace()
        .map(str::trim)
        .filter(|entry| !entry.is_empty())
        .map(ToString::to_string)
        .collect()
}

fn printable_units(units: &[String]) -> String {
    if units.is_empty() {
        "-".to_string()
    } else {
        units.join(" ")
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
    quiet_stop_units: &[String],
    quiet_start_units: &[String],
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

        let previous_state = *quiet_state;
        let next_state = next_quiet_state(*quiet_state, &event);
        if quiet_service_mode(previous_state) != quiet_service_mode(next_state) {
            reconcile_quiet_services(
                quiet_service_mode(next_state),
                quiet_stop_units,
                quiet_start_units,
            )?;
        }
        *quiet_state = next_state;
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

fn quiet_service_mode(state: QuietModeState) -> bool {
    matches!(state, QuietModeState::Prepare | QuietModeState::Active)
}

fn reconcile_quiet_services(
    quiet_active: bool,
    quiet_stop_units: &[String],
    quiet_start_units: &[String],
) -> Result<(), String> {
    if quiet_active {
        run_systemctl("stop", quiet_stop_units)
    } else {
        run_systemctl("start", quiet_start_units)
    }
}

fn run_systemctl(action: &str, units: &[String]) -> Result<(), String> {
    if units.is_empty() {
        return Ok(());
    }

    let status = Command::new("systemctl")
        .arg(action)
        .args(units)
        .status()
        .map_err(|err| format!("systemctl {action} {}: {err}", units.join(" ")))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!(
            "systemctl {action} {} exited with {status}",
            units.join(" ")
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::{next_quiet_state, parse_service_list, quiet_service_mode, QuietModeState};
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

    #[test]
    fn playback_failure_restores_quiet_service_mode() {
        let state = next_quiet_state(
            QuietModeState::Active,
            &PlaybackEvent::PlaybackFailed {
                reason: "alsa_open_failed".to_string(),
                class: PlaybackFailureClass::Output,
                recoverable: false,
                keep_quiet: false,
            },
        );

        assert!(!quiet_service_mode(state));
    }

    #[test]
    fn error_hold_does_not_keep_quiet_service_mode() {
        assert!(!quiet_service_mode(QuietModeState::ErrorHold));
    }

    #[test]
    fn parses_service_lists_from_env_string() {
        assert_eq!(
            parse_service_list("bluetooth.service lumelo-wifi-provisiond.service"),
            vec![
                "bluetooth.service".to_string(),
                "lumelo-wifi-provisiond.service".to_string()
            ]
        );
    }
}
