use std::env;
use std::fs;
use std::io::{BufRead, BufReader};
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
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
    let protected_services =
        parse_service_list(&env::var("SESSIOND_PROTECTED_SERVICES").unwrap_or_default());
    let freezable_units =
        parse_service_list(&env::var("SESSIOND_FREEZABLE_SERVICES").unwrap_or_default());
    let quiet_stop_units =
        parse_service_list(&env::var("SESSIOND_QUIET_STOP_UNITS").unwrap_or_default());
    let quiet_start_units =
        parse_service_list(&env::var("SESSIOND_QUIET_START_UNITS").unwrap_or_default());
    let quiet_mdns_interfaces =
        MdnsInterfaceConfig::parse(&env::var("SESSIOND_QUIET_MDNS_INTERFACES").unwrap_or_default());
    validate_protected_units(
        &protected_services,
        &quiet_stop_units,
        &quiet_start_units,
        &freezable_units,
    )?;

    let mut quiet_reconciler = QuietReconciler::new(
        env::var("SESSIOND_SYSTEMCTL").unwrap_or_else(|_| "systemctl".to_string()),
        env::var("SESSIOND_RESOLVECTL").unwrap_or_else(|_| "resolvectl".to_string()),
        quiet_stop_units.clone(),
        quiet_start_units.clone(),
        freezable_units.clone(),
        quiet_mdns_interfaces.clone(),
    );
    let mut quiet_state = QuietModeState::Off;

    println!("sessiond watching");
    println!("  event source:   {}", event_socket.display());
    println!("  quiet flag:     {}", quiet_flag.display());
    println!("  protected svc:  {}", printable_units(&protected_services));
    println!("  freezable svc:  {}", printable_units(&freezable_units));
    println!("  quiet stop:     {}", printable_units(&quiet_stop_units));
    println!("  quiet start:    {}", printable_units(&quiet_start_units));
    println!("  quiet mdns:     {}", quiet_mdns_interfaces.describe());

    loop {
        match UnixStream::connect(&event_socket) {
            Ok(stream) => {
                if let Err(err) =
                    handle_stream(stream, &quiet_flag, &mut quiet_reconciler, &mut quiet_state)
                {
                    eprintln!("sessiond stream error: {err}");
                }
            }
            Err(err) => eprintln!("sessiond connect retry: {err}"),
        }

        thread::sleep(Duration::from_millis(300));
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

fn dedupe_units(units: impl IntoIterator<Item = String>) -> Vec<String> {
    let mut deduped = Vec::new();
    for unit in units {
        if !deduped.contains(&unit) {
            deduped.push(unit);
        }
    }
    deduped
}

fn validate_protected_units(
    protected_units: &[String],
    quiet_stop_units: &[String],
    quiet_start_units: &[String],
    freezable_units: &[String],
) -> Result<(), String> {
    for unit in quiet_stop_units
        .iter()
        .chain(quiet_start_units)
        .chain(freezable_units)
    {
        if protected_units.contains(unit) {
            return Err(format!(
                "protected unit {unit} cannot be managed by Quiet Mode"
            ));
        }
    }
    Ok(())
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
    quiet_reconciler: &mut QuietReconciler,
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
            quiet_reconciler.reconcile(quiet_service_mode(next_state))?;
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
    matches!(
        state,
        QuietModeState::Prepare | QuietModeState::Active | QuietModeState::ErrorHold
    )
}

#[derive(Debug, Clone, Eq, PartialEq)]
enum MdnsInterfaceConfig {
    Disabled,
    Auto,
    Interfaces(Vec<String>),
}

impl MdnsInterfaceConfig {
    fn parse(raw: &str) -> Self {
        let entries = parse_service_list(raw);
        if entries.is_empty() || entries.iter().any(|entry| entry == "off") {
            Self::Disabled
        } else if entries.iter().any(|entry| entry == "auto") {
            Self::Auto
        } else {
            Self::Interfaces(entries)
        }
    }

    fn describe(&self) -> String {
        match self {
            Self::Disabled => "-".to_string(),
            Self::Auto => "auto".to_string(),
            Self::Interfaces(interfaces) => printable_units(interfaces),
        }
    }

    fn resolve_interfaces(&self) -> Result<Vec<String>, String> {
        match self {
            Self::Disabled => Ok(Vec::new()),
            Self::Interfaces(interfaces) => Ok(interfaces
                .iter()
                .filter(|iface| interface_exists(iface))
                .cloned()
                .collect()),
            Self::Auto => {
                let entries = fs::read_dir("/sys/class/net").map_err(|err| {
                    format!("list network interfaces for mDNS suppression: {err}")
                })?;
                let mut interfaces = Vec::new();
                for entry in entries {
                    let entry = entry.map_err(|err| {
                        format!("read network interface for mDNS suppression: {err}")
                    })?;
                    let iface = entry.file_name().to_string_lossy().to_string();
                    if iface != "lo" {
                        interfaces.push(iface);
                    }
                }
                interfaces.sort();
                Ok(interfaces)
            }
        }
    }
}

#[derive(Debug, Clone)]
struct MdnsInterfaceSnapshot {
    interface: String,
    mode: String,
}

#[derive(Debug, Clone)]
struct QuietReconcileSnapshot {
    active_units: Vec<String>,
    mdns_interfaces: Vec<MdnsInterfaceSnapshot>,
}

struct QuietReconciler {
    systemctl: String,
    resolvectl: String,
    stop_units: Vec<String>,
    restore_candidates: Vec<String>,
    mdns_interfaces: MdnsInterfaceConfig,
    snapshot: Option<QuietReconcileSnapshot>,
}

impl QuietReconciler {
    fn new(
        systemctl: String,
        resolvectl: String,
        quiet_stop_units: Vec<String>,
        quiet_start_units: Vec<String>,
        freezable_units: Vec<String>,
        mdns_interfaces: MdnsInterfaceConfig,
    ) -> Self {
        let stop_units = dedupe_units(quiet_stop_units.into_iter().chain(freezable_units));
        let restore_candidates = dedupe_units(stop_units.iter().cloned().chain(quiet_start_units));

        Self {
            systemctl,
            resolvectl,
            stop_units,
            restore_candidates,
            mdns_interfaces,
            snapshot: None,
        }
    }

    fn reconcile(&mut self, quiet_active: bool) -> Result<(), String> {
        if quiet_active {
            self.enter_quiet()
        } else {
            self.exit_quiet()
        }
    }

    fn enter_quiet(&mut self) -> Result<(), String> {
        if self.snapshot.is_some() {
            return Ok(());
        }

        let mut active_units = Vec::new();
        for unit in &self.restore_candidates {
            if systemctl_is_active(&self.systemctl, unit)? {
                active_units.push(unit.clone());
            }
        }

        for unit in &self.stop_units {
            if active_units.contains(unit) {
                run_systemctl(&self.systemctl, "stop", &[unit.clone()])?;
                println!("sessiond quiet stopped {unit}");
            }
        }

        let mdns_interfaces = self.suppress_mdns()?;
        self.snapshot = Some(QuietReconcileSnapshot {
            active_units,
            mdns_interfaces,
        });
        Ok(())
    }

    fn exit_quiet(&mut self) -> Result<(), String> {
        let Some(snapshot) = self.snapshot.take() else {
            return Ok(());
        };

        self.restore_mdns(&snapshot.mdns_interfaces)?;
        for unit in snapshot.active_units {
            run_systemctl(&self.systemctl, "start", &[unit.clone()])?;
            println!("sessiond quiet restored {unit}");
        }
        Ok(())
    }

    fn suppress_mdns(&self) -> Result<Vec<MdnsInterfaceSnapshot>, String> {
        let interfaces = self.mdns_interfaces.resolve_interfaces()?;
        let mut snapshots = Vec::new();
        for interface in interfaces {
            let mode = resolvectl_get_mdns(&self.resolvectl, &interface)?;
            if mode != "no" {
                resolvectl_set_mdns(&self.resolvectl, &interface, "no")?;
                println!("sessiond quiet suppressed mDNS on {interface}");
            }
            snapshots.push(MdnsInterfaceSnapshot { interface, mode });
        }
        Ok(snapshots)
    }

    fn restore_mdns(&self, snapshots: &[MdnsInterfaceSnapshot]) -> Result<(), String> {
        for snapshot in snapshots {
            if interface_exists(&snapshot.interface) {
                resolvectl_set_mdns(&self.resolvectl, &snapshot.interface, &snapshot.mode)?;
                println!(
                    "sessiond quiet restored mDNS on {} to {}",
                    snapshot.interface, snapshot.mode
                );
            }
        }
        Ok(())
    }
}

fn interface_exists(interface: &str) -> bool {
    PathBuf::from("/sys/class/net").join(interface).exists()
}

fn systemctl_is_active(systemctl: &str, unit: &str) -> Result<bool, String> {
    let status = Command::new(systemctl)
        .arg("is-active")
        .arg("--quiet")
        .arg(unit)
        .status()
        .map_err(|err| format!("{systemctl} is-active --quiet {unit}: {err}"))?;
    Ok(status.success())
}

fn run_systemctl(systemctl: &str, action: &str, units: &[String]) -> Result<(), String> {
    if units.is_empty() {
        return Ok(());
    }

    let status = Command::new(systemctl)
        .arg(action)
        .args(units)
        .status()
        .map_err(|err| format!("{systemctl} {action} {}: {err}", units.join(" ")))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!(
            "{systemctl} {action} {} exited with {status}",
            units.join(" ")
        ))
    }
}

fn resolvectl_get_mdns(resolvectl: &str, interface: &str) -> Result<String, String> {
    let output = Command::new(resolvectl)
        .arg("mdns")
        .arg(interface)
        .output()
        .map_err(|err| format!("{resolvectl} mdns {interface}: {err}"))?;
    if !output.status.success() {
        return Err(format!(
            "{resolvectl} mdns {interface} exited with {}: {}",
            output.status,
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    parse_resolvectl_mdns_mode(&String::from_utf8_lossy(&output.stdout))
        .ok_or_else(|| format!("could not parse {resolvectl} mdns {interface} output"))
}

fn resolvectl_set_mdns(resolvectl: &str, interface: &str, mode: &str) -> Result<(), String> {
    let status = Command::new(resolvectl)
        .arg("mdns")
        .arg(interface)
        .arg(mode)
        .status()
        .map_err(|err| format!("{resolvectl} mdns {interface} {mode}: {err}"))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!(
            "{resolvectl} mdns {interface} {mode} exited with {status}"
        ))
    }
}

fn parse_resolvectl_mdns_mode(output: &str) -> Option<String> {
    output
        .split_whitespace()
        .rev()
        .find(|token| matches!(*token, "yes" | "no" | "resolve"))
        .map(ToString::to_string)
}

#[cfg(test)]
mod tests {
    use super::{
        next_quiet_state, parse_resolvectl_mdns_mode, parse_service_list, quiet_service_mode,
        validate_protected_units, MdnsInterfaceConfig, QuietModeState,
    };
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
                auto_skip_after_ms: Some(6000),
                queue_entry_id: Some("q1".to_string()),
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
                auto_skip_after_ms: None,
                queue_entry_id: None,
            },
        );

        assert!(!quiet_service_mode(state));
    }

    #[test]
    fn error_hold_keeps_quiet_service_mode() {
        assert!(quiet_service_mode(QuietModeState::ErrorHold));
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

    #[test]
    fn protected_units_cannot_be_quiet_managed() {
        let err = validate_protected_units(
            &["controld.service".to_string()],
            &["controld.service".to_string()],
            &[],
            &[],
        )
        .unwrap_err();

        assert!(err.contains("protected unit controld.service"));
    }

    #[test]
    fn mdns_interface_config_supports_auto_off_and_explicit_interfaces() {
        assert_eq!(
            MdnsInterfaceConfig::parse(""),
            MdnsInterfaceConfig::Disabled
        );
        assert_eq!(
            MdnsInterfaceConfig::parse("auto"),
            MdnsInterfaceConfig::Auto
        );
        assert_eq!(
            MdnsInterfaceConfig::parse("eth0 wlan0"),
            MdnsInterfaceConfig::Interfaces(vec!["eth0".to_string(), "wlan0".to_string()])
        );
    }

    #[test]
    fn parses_resolvectl_mdns_modes() {
        assert_eq!(
            parse_resolvectl_mdns_mode("Link 2 (eth0): yes\n"),
            Some("yes".to_string())
        );
        assert_eq!(
            parse_resolvectl_mdns_mode("Global: resolve\nLink 3 (wlan0): no\n"),
            Some("no".to_string())
        );
    }
}
