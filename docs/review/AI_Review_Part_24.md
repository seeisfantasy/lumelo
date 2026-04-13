# AI Review Part 24

这是给外部 AI 做静态审查的代码分卷。每一卷都只包含仓库快照中的一部分文本文件内容，按当前工作树生成。

## `services/rust/crates/playbackd/src/main.rs` (2/3)

- bytes: 84527
- segment: 2/3

~~~rust
            persist_history: true,
        }
    }

    fn status_snapshot(&self) -> PlaybackStatusSnapshot {
        PlaybackStatusSnapshot {
            state: self.playback_state,
            order_mode: order_mode_label(self.snapshot.order_mode).to_string(),
            repeat_mode: repeat_mode_label(self.snapshot.repeat_mode).to_string(),
            current_track: self.current_track.clone(),
            last_command: self.last_command.clone(),
            queue_entries: self.snapshot.tracks.len(),
        }
    }

    fn queue_snapshot_view(&self) -> QueueSnapshotView {
        let entries = self
            .snapshot
            .play_order
            .iter()
            .enumerate()
            .filter_map(|(order_index, queue_entry_id)| {
                self.snapshot
                    .tracks
                    .iter()
                    .find(|entry| &entry.queue_entry_id == queue_entry_id)
                    .map(|entry| QueueSnapshotEntryView {
                        order_index,
                        queue_entry_id: entry.queue_entry_id.clone(),
                        track_uid: entry.track_uid.clone(),
                        volume_uuid: entry.volume_uuid.clone(),
                        relative_path: entry.relative_path.clone(),
                        title: entry.title.clone(),
                        duration_ms: entry.duration_ms,
                        is_current: self.snapshot.current_order_index == Some(order_index),
                    })
            })
            .collect();

        QueueSnapshotView {
            order_mode: order_mode_label(self.snapshot.order_mode).to_string(),
            repeat_mode: repeat_mode_label(self.snapshot.repeat_mode).to_string(),
            current_order_index: self.snapshot.current_order_index,
            entries,
        }
    }

    fn resolved_current_track(&self) -> Option<String> {
        resolve_track_uid(&self.snapshot, self.snapshot.current_order_index)
    }

    fn push_track_entry(&mut self, track_id: String) -> String {
        let queue_entry_id = format!("q{}", self.next_queue_entry_id);
        self.next_queue_entry_id += 1;

        self.snapshot.tracks.push(QueueEntry {
            queue_entry_id: queue_entry_id.clone(),
            track_uid: track_id.clone(),
            volume_uuid: "manual".to_string(),
            relative_path: track_id.clone(),
            title: Some(track_id),
            duration_ms: None,
        });

        queue_entry_id
    }

    fn ensure_selected_track_for_non_empty_queue(&mut self) {
        if self.snapshot.play_order.is_empty() {
            self.snapshot.current_order_index = None;
            self.current_track = None;
            self.playback_state = PlaybackState::Idle;
            return;
        }

        if self.snapshot.current_order_index.is_none() {
            self.snapshot.current_order_index = Some(0);
        }

        self.current_track = self.resolved_current_track();
        if !self.has_active_output() {
            self.playback_state = PlaybackState::Stopped;
        }
    }

    fn has_active_output(&self) -> bool {
        matches!(
            self.playback_state,
            PlaybackState::PreQuiet | PlaybackState::QuietActive
        )
    }

    fn record_current_track_history(&mut self) {
        if let Some(entry) = self.current_history_entry() {
            self.history_log.push_recent(entry, HISTORY_LIMIT);
        }
    }

    fn current_history_entry(&self) -> Option<HistoryEntry> {
        let queue_entry = resolve_queue_entry(&self.snapshot, self.snapshot.current_order_index)?;
        Some(HistoryEntry {
            played_at: unix_timestamp_secs(),
            track_uid: queue_entry.track_uid,
            volume_uuid: queue_entry.volume_uuid,
            relative_path: queue_entry.relative_path,
            title: queue_entry.title,
            duration_ms: queue_entry.duration_ms,
        })
    }

    fn enrich_current_track(&mut self, track: &ResolvedTrack) -> bool {
        let Some(current_queue_entry_id) = self
            .snapshot
            .current_order_index
            .and_then(|index| self.snapshot.play_order.get(index))
            .cloned()
        else {
            return false;
        };

        let Some(entry) = self
            .snapshot
            .tracks
            .iter_mut()
            .find(|entry| entry.queue_entry_id == current_queue_entry_id)
        else {
            return false;
        };

        let mut changed = false;
        if entry.track_uid != track.track_uid {
            entry.track_uid = track.track_uid.clone();
            changed = true;
        }
        if entry.volume_uuid != track.volume_uuid {
            entry.volume_uuid = track.volume_uuid.clone();
            changed = true;
        }
        if entry.relative_path != track.relative_path {
            entry.relative_path = track.relative_path.clone();
            changed = true;
        }
        if entry.title != track.title {
            entry.title = track.title.clone();
            changed = true;
        }
        if entry.duration_ms != track.duration_ms {
            entry.duration_ms = track.duration_ms;
            changed = true;
        }

        changed
    }

    fn mark_output_failure(&mut self, track_id: &str, reason: &str) -> bool {
        if self.current_track.as_deref() != Some(track_id) {
            return false;
        }
        if !matches!(
            self.playback_state,
            PlaybackState::QuietActive | PlaybackState::Paused
        ) {
            return false;
        }

        self.playback_state = PlaybackState::QuietErrorHold;
        self.last_command = Some(format!("playback_failed:{track_id}:{reason}"));
        true
    }

    fn finish_track_output(&mut self, track_id: &str) -> TrackFinishAction {
        if self.current_track.as_deref() != Some(track_id) {
            return TrackFinishAction::None;
        }
        if self.playback_state != PlaybackState::QuietActive {
            return TrackFinishAction::None;
        }

        if let Some(next_index) = self.next_index_after_track_finish() {
            self.snapshot.current_order_index = Some(next_index);
            let Some(next_track) = self.resolved_current_track() else {
                self.playback_state = PlaybackState::Stopped;
                self.current_track = None;
                self.last_command = Some(format!("finished:{track_id}:missing_next"));
                return TrackFinishAction::Stop {
                    reason: "track_finished".to_string(),
                };
            };

            self.current_track = Some(next_track.clone());
            self.playback_state = PlaybackState::QuietActive;
            self.last_command = Some(format!("auto_next:{next_track}"));
            self.record_current_track_history();
            return TrackFinishAction::Start {
                track_id: next_track,
            };
        }

        self.playback_state = PlaybackState::Stopped;
        self.last_command = Some(format!("finished:{track_id}"));
        TrackFinishAction::Stop {
            reason: "track_finished".to_string(),
        }
    }

    fn next_index_after_track_finish(&self) -> Option<usize> {
        let current_index = self.snapshot.current_order_index?;
        let queue_len = self.snapshot.play_order.len();
        if queue_len == 0 || current_index >= queue_len {
            return None;
        }

        match self.snapshot.repeat_mode {
            RepeatMode::Off => (current_index + 1 < queue_len).then_some(current_index + 1),
            RepeatMode::One => Some(current_index),
            RepeatMode::All => Some((current_index + 1) % queue_len),
        }
    }
}

#[derive(Clone, Debug)]
struct EventHub {
    subscribers: Arc<Mutex<Vec<UnixStream>>>,
}

impl EventHub {
    fn bind(path: &Path) -> Result<Self, String> {
        prepare_socket_path(path)?;
        let listener =
            UnixListener::bind(path).map_err(|err| format!("bind {}: {err}", path.display()))?;
        let subscribers = Arc::new(Mutex::new(Vec::new()));
        let acceptors = Arc::clone(&subscribers);

        thread::spawn(move || {
            for stream in listener.incoming() {
                match stream {
                    Ok(stream) => {
                        if let Err(err) = stream.set_nonblocking(true) {
                            eprintln!("playbackd event subscriber nonblocking error: {err}");
                            continue;
                        }

                        match acceptors.lock() {
                            Ok(mut subscribers) => subscribers.push(stream),
                            Err(_) => {
                                eprintln!("playbackd event subscriber lock poisoned");
                                break;
                            }
                        }
                    }
                    Err(err) => eprintln!("playbackd event accept error: {err}"),
                }
            }
        });

        Ok(Self { subscribers })
    }

    fn broadcast(&self, event: &PlaybackEvent) -> Result<(), String> {
        let line = format_event_line(event);
        let mut subscribers = self
            .subscribers
            .lock()
            .map_err(|_| "event subscriber lock poisoned".to_string())?;

        subscribers.retain_mut(|stream| {
            stream.write_all(line.as_bytes()).is_ok() && stream.write_all(b"\n").is_ok()
        });

        Ok(())
    }
}

fn handle_client(
    stream: UnixStream,
    state: Arc<Mutex<RuntimeState>>,
    event_hub: EventHub,
    queue_path: Arc<PathBuf>,
    history_path: Arc<PathBuf>,
    track_resolver: TrackResolver,
    output_controller: OutputController,
) -> Result<(), String> {
    let mut reader = BufReader::new(
        stream
            .try_clone()
            .map_err(|err| format!("clone client stream: {err}"))?,
    );
    let mut line = String::new();
    let read = reader
        .read_line(&mut line)
        .map_err(|err| format!("read command: {err}"))?;
    if read == 0 {
        return Err("client disconnected before sending a command".to_string());
    }

    let command = match parse_command_line(&line) {
        Ok(command) => command,
        Err(err) => {
            let response = format_error_line(err.code, &err.message);
            let mut stream = stream;
            stream
                .write_all(format!("{response}\n").as_bytes())
                .map_err(|write_err| format!("write parse failure: {write_err}"))?;
            return Ok(());
        }
    };

    let (mut outcome, mut snapshot_to_persist, history_to_persist, output_action) = {
        let mut state = state
            .lock()
            .map_err(|_| "playback state lock poisoned".to_string())?;
        let outcome = state.apply_command(command);
        let output_action = derive_output_action(&state, &outcome);
        let snapshot = outcome.persist_queue.then(|| state.snapshot.clone());
        let history = outcome.persist_history.then(|| state.history_log.clone());
        (outcome, snapshot, history, output_action)
    };

    apply_output_action(
        Arc::clone(&state),
        &event_hub,
        &output_controller,
        &track_resolver,
        Arc::clone(&queue_path),
        Arc::clone(&history_path),
        output_action,
        &mut outcome,
        &mut snapshot_to_persist,
    );

    if let Some(snapshot) = snapshot_to_persist {
        persist_queue_snapshot(queue_path.as_ref(), &snapshot)?;
    }
    if let Some(history_log) = history_to_persist {
        persist_history_log(history_path.as_ref(), &history_log)?;
    }

    let mut stream = stream;
    stream
        .write_all(format!("{}\n", outcome.response_line).as_bytes())
        .map_err(|err| format!("write response: {err}"))?;

    for event in &outcome.events {
        if let Err(err) = event_hub.broadcast(event) {
            eprintln!("playbackd event broadcast error: {err}");
        }
    }

    Ok(())
}

fn derive_output_action(state: &RuntimeState, outcome: &CommandOutcome) -> OutputAction {
    for event in &outcome.events {
        match event {
            PlaybackEvent::PlaybackStarted { track_id } => {
                return OutputAction::Start(track_id.clone())
            }
            PlaybackEvent::TrackChanged { track_id } => {
                return OutputAction::Start(track_id.clone())
            }
            PlaybackEvent::PlaybackPaused => return OutputAction::Pause,
            PlaybackEvent::PlaybackResumed => {
                if let Some(track_id) = state.current_track.clone() {
                    return OutputAction::Resume(track_id);
                }
            }
            PlaybackEvent::PlaybackStopped { .. } => return OutputAction::Stop,
            PlaybackEvent::PlayRequestAccepted { .. } | PlaybackEvent::PlaybackFailed { .. } => {}
        }
    }

    OutputAction::None
}

fn apply_output_action(
    state: Arc<Mutex<RuntimeState>>,
    event_hub: &EventHub,
    output_controller: &OutputController,
    track_resolver: &TrackResolver,
    queue_path: Arc<PathBuf>,
    history_path: Arc<PathBuf>,
    output_action: OutputAction,
    outcome: &mut CommandOutcome,
    snapshot_to_persist: &mut Option<QueueSnapshot>,
) {
    match output_action {
        OutputAction::None => {}
        OutputAction::Stop => output_controller.stop(),
        OutputAction::Pause => {
            if let Err(err) = output_controller.pause() {
                apply_operation_failure(state, outcome, None, err);
            }
        }
        OutputAction::Resume(track_id) => {
            if let Err(err) = output_controller.resume(&track_id) {
                apply_operation_failure(state, outcome, Some(track_id), err);
            }
        }
        OutputAction::Start(track_id) => {
            let track = match track_resolver.resolve(&track_id) {
                Ok(track) => track,
                Err(err) => {
                    apply_operation_failure(state, outcome, Some(track_id), err);
                    return;
                }
            };

            if let Ok(mut runtime) = state.lock() {
                if runtime.enrich_current_track(&track) {
                    *snapshot_to_persist = Some(runtime.snapshot.clone());
                }
            }

            if let Err(err) = output_controller.start(
                track,
                Arc::clone(&state),
                event_hub.clone(),
                queue_path,
                history_path,
                track_resolver.clone(),
            ) {
                apply_operation_failure(state, outcome, Some(track_id), err);
            }
        }
    }
}

fn apply_operation_failure(
    state: Arc<Mutex<RuntimeState>>,
    outcome: &mut CommandOutcome,
    track_id: Option<String>,
    err: PlaybackOperationError,
) {
    if let Some(track_id) = track_id.as_deref() {
        if let Ok(mut runtime) = state.lock() {
            let _ = runtime.mark_output_failure(track_id, &err.reason);
        }
    }

    outcome.response_line = format_error_line(err.code, &err.reason);
    outcome.events = vec![PlaybackEvent::PlaybackFailed {
        reason: err.code.to_string(),
        class: err.class,
        recoverable: err.recoverable,
        keep_quiet: err.keep_quiet,
    }];
}

fn apply_async_operation_failure(
    state: Arc<Mutex<RuntimeState>>,
    event_hub: &EventHub,
    track_id: Option<&str>,
    err: PlaybackOperationError,
) {
    if let Some(track_id) = track_id {
        if let Ok(mut runtime) = state.lock() {
            let _ = runtime.mark_output_failure(track_id, &err.reason);
        }
    }

    let _ = event_hub.broadcast(&PlaybackEvent::PlaybackFailed {
        reason: err.code.to_string(),
        class: err.class,
        recoverable: err.recoverable,
        keep_quiet: err.keep_quiet,
    });
}

fn handle_finished_track_success(
    output_controller: &OutputController,
    state: Arc<Mutex<RuntimeState>>,
    event_hub: &EventHub,
    queue_path: Arc<PathBuf>,
    history_path: Arc<PathBuf>,
    track_resolver: TrackResolver,
    finished_track_id: &str,
) {
    let (finish_action, queue_snapshot, history_log) = {
        let mut runtime = match state.lock() {
            Ok(runtime) => runtime,
            Err(_) => return,
        };
        let finish_action = runtime.finish_track_output(finished_track_id);
        let queue_snapshot = matches!(finish_action, TrackFinishAction::Start { .. })
            .then(|| runtime.snapshot.clone());
        let history_log = matches!(finish_action, TrackFinishAction::Start { .. })
            .then(|| runtime.history_log.clone());
        (finish_action, queue_snapshot, history_log)
    };

    if let Some(snapshot) = queue_snapshot.as_ref() {
        if let Err(err) = persist_queue_snapshot(queue_path.as_ref(), snapshot) {
            eprintln!("playbackd async queue persist error: {err}");
        }
    }
    if let Some(history_log) = history_log.as_ref() {
        if let Err(err) = persist_history_log(history_path.as_ref(), history_log) {
            eprintln!("playbackd async history persist error: {err}");
        }
    }

    match finish_action {
        TrackFinishAction::None => {}
        TrackFinishAction::Stop { reason } => {
            let _ = event_hub.broadcast(&PlaybackEvent::PlaybackStopped { reason });
        }
        TrackFinishAction::Start { track_id } => {
            let track = match track_resolver.resolve(&track_id) {
                Ok(track) => track,
                Err(err) => {
                    apply_async_operation_failure(state, event_hub, Some(track_id.as_str()), err);
                    return;
                }
            };

            let maybe_snapshot = {
                let mut runtime = match state.lock() {
                    Ok(runtime) => runtime,
                    Err(_) => return,
                };
                runtime
                    .enrich_current_track(&track)
                    .then(|| runtime.snapshot.clone())
            };
            if let Some(snapshot) = maybe_snapshot.as_ref() {
                if let Err(err) = persist_queue_snapshot(queue_path.as_ref(), snapshot) {
                    eprintln!("playbackd async queue persist error: {err}");
                }
            }

            if let Err(err) = output_controller.start(
                track,
                Arc::clone(&state),
                event_hub.clone(),
                Arc::clone(&queue_path),
                Arc::clone(&history_path),
                track_resolver,
            ) {
                apply_async_operation_failure(state, event_hub, Some(track_id.as_str()), err);
                return;
            }

            let _ = event_hub.broadcast(&PlaybackEvent::TrackChanged { track_id });
        }
    }
}

fn prepare_socket_path(path: &Path) -> Result<(), String> {
    let parent = path
        .parent()
        .ok_or_else(|| format!("socket path has no parent: {}", path.display()))?;
    fs::create_dir_all(parent)
        .map_err(|err| format!("create socket dir {}: {err}", parent.display()))?;

    if path.exists() {
        fs::remove_file(path)
            .map_err(|err| format!("remove stale socket {}: {err}", path.display()))?;
    }

    Ok(())
}

fn load_runtime_state(queue_path: &Path, history_path: &Path) -> RuntimeState {
    let snapshot = match load_queue_snapshot(queue_path) {
        Ok(snapshot) => snapshot,
        Err(err) => {
            eprintln!("playbackd queue restore fallback: {err}");
            QueueSnapshot::empty()
        }
    };
    let history_log = match load_history_log(history_path) {
        Ok(history_log) => history_log,
        Err(err) => {
            eprintln!("playbackd history restore fallback: {err}");
            HistoryLog::empty()
        }
    };

    if snapshot.tracks.is_empty() {
        let mut state = RuntimeState::new();
        state.history_log = history_log;
        state
    } else {
        RuntimeState::restore(snapshot, history_log)
    }
}

fn load_queue_snapshot(queue_path: &Path) -> Result<QueueSnapshot, String> {
    if !queue_path.exists() {
        return Ok(QueueSnapshot::empty());
    }

    let raw = fs::read_to_string(queue_path)
        .map_err(|err| format!("read queue state {}: {err}", queue_path.display()))?;
    let persisted: PersistedQueueState = serde_json::from_str(&raw)
        .map_err(|err| format!("parse queue state {}: {err}", queue_path.display()))?;
    if persisted.version != QUEUE_FILE_VERSION {
        return Err(format!(
            "unsupported queue state version {} in {}",
            persisted.version,
            queue_path.display()
        ));
    }

    Ok(persisted.into_snapshot())
}

fn load_history_log(history_path: &Path) -> Result<HistoryLog, String> {
    if !history_path.exists() {
        return Ok(HistoryLog::empty());
    }

    let raw = fs::read_to_string(history_path)
        .map_err(|err| format!("read history state {}: {err}", history_path.display()))?;
    let persisted: PersistedHistoryState = serde_json::from_str(&raw)
        .map_err(|err| format!("parse history state {}: {err}", history_path.display()))?;
    if persisted.version != HISTORY_FILE_VERSION {
        return Err(format!(
            "unsupported history state version {} in {}",
            persisted.version,
            history_path.display()
        ));
    }

    Ok(persisted.into_log())
}

fn persist_queue_snapshot(queue_path: &Path, snapshot: &QueueSnapshot) -> Result<(), String> {
    let persisted = PersistedQueueState::from_snapshot(snapshot);
    persist_state_file(queue_path, &persisted, "queue state")
}

fn persist_history_log(history_path: &Path, history_log: &HistoryLog) -> Result<(), String> {
    let persisted = PersistedHistoryState::from_log(history_log);
    persist_state_file(history_path, &persisted, "history state")
}

fn persist_state_file<T>(path: &Path, value: &T, label: &str) -> Result<(), String>
where
    T: Serialize,
{
    let parent = path
        .parent()
        .ok_or_else(|| format!("{label} path has no parent: {}", path.display()))?;
    fs::create_dir_all(parent)
        .map_err(|err| format!("create {label} dir {}: {err}", parent.display()))?;

    let encoded = serde_json::to_string_pretty(value)
        .map_err(|err| format!("encode {label} {}: {err}", path.display()))?;
    let tmp_path = unique_state_tmp_path(path);
    let mut file = fs::File::create(&tmp_path)
        .map_err(|err| format!("create {label} temp {}: {err}", tmp_path.display()))?;
    file.write_all(format!("{encoded}\n").as_bytes())
        .map_err(|err| format!("write {label} temp {}: {err}", tmp_path.display()))?;
    file.sync_all()
        .map_err(|err| format!("fsync {label} temp {}: {err}", tmp_path.display()))?;
    fs::rename(&tmp_path, path).map_err(|err| {
        format!(
            "replace {label} {} from {}: {err}",
            path.display(),
            tmp_path.display()
        )
    })?;

    Ok(())
}

fn unique_state_tmp_path(path: &Path) -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let file_name = path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("state.json");

    path.with_file_name(format!("{file_name}.tmp.{nonce}"))
}

fn send_signal(pid: u32, signal: &str) -> Result<(), PlaybackOperationError> {
    let status = Command::new("kill")
        .arg(format!("-{signal}"))
        .arg(pid.to_string())
        .status()
        .map_err(|err| {
            PlaybackOperationError::output(
                "signal_failed",
                format!("send SIG{signal} to pid {pid}: {err}"),
            )
        })?;

    if status.success() {
        return Ok(());
    }

    Err(PlaybackOperationError::output(
        "signal_failed",
        format!(
            "send SIG{signal} to pid {pid}: {}",
            exit_status_label(&status)
        ),
    ))
}

fn wait_for_process_exit(pid: u32, timeout: Duration) -> bool {
    let started = std::time::Instant::now();
    loop {
        if !process_exists(pid) {
            return true;
        }
        if started.elapsed() >= timeout {
            return false;
        }
        thread::sleep(Duration::from_millis(25));
    }
}

fn process_exists(pid: u32) -> bool {
    Command::new("kill")
        .arg("-0")
        .arg(pid.to_string())
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

fn resolved_format(track: &ResolvedTrack) -> String {
    if !track.format.is_empty() {
        return track.format.clone();
    }

    detect_format(&track.absolute_path)
}

fn inspect_decoded_pcm(track: &ResolvedTrack) -> Result<DecodedPCMConfig, PlaybackOperationError> {
    let (mut format, track_id, mut decoder) = open_decoder(&track.absolute_path).map_err(|err| {
        PlaybackOperationError::content(
            "unsupported_format",
            format!(
                "prepare decoder for {} ({}): {err}",
                track.track_uid,
                empty_label(&resolved_format(track))
            ),
            true,
        )
    })?;

    loop {
        let packet = match format.next_packet() {
            Ok(packet) => packet,
            Err(SymphoniaError::IoError(err))
                if err.kind() == std::io::ErrorKind::UnexpectedEof =>
            {
                return Err(PlaybackOperationError::content(
                    "decode_probe_failed",
                    format!("decoded stream ended before yielding audio: {}", track.track_uid),
                    true,
                ));
            }
            Err(err) => {
                return Err(PlaybackOperationError::content(
                    "decode_probe_failed",
                    format!("read decoded packet for {}: {err}", track.track_uid),
                    true,
                ));
            }
        };

        if packet.track_id() != track_id {
            continue;
        }

        match decoder.decode(&packet) {
            Ok(decoded) => {
                return Ok(DecodedPCMConfig {
                    sample_rate: decoded.spec().rate,
                    channels: decoded.spec().channels.count(),
                });
            }
            Err(SymphoniaError::DecodeError(_)) => continue,
            Err(err) => {
                return Err(PlaybackOperationError::content(
                    "decode_probe_failed",
                    format!("decode first audio packet for {}: {err}", track.track_uid),
                    true,
                ));
            }
        }
    }
}

fn stream_decoded_audio(path: PathBuf, mut stdin: ChildStdin) -> Result<(), String> {
    let (mut format, track_id, mut decoder) = open_decoder(&path)?;
    let mut pcm_bytes = Vec::new();

    loop {
        let packet = match format.next_packet() {
            Ok(packet) => packet,
            Err(SymphoniaError::IoError(err))
                if err.kind() == std::io::ErrorKind::UnexpectedEof =>
            {
                break;
            }
            Err(err) => return Err(format!("read decoded packet for {}: {err}", path.display())),
        };

        if packet.track_id() != track_id {
            continue;
        }

        let decoded = match decoder.decode(&packet) {
            Ok(decoded) => decoded,
            Err(SymphoniaError::DecodeError(_)) => continue,
            Err(SymphoniaError::IoError(err))
                if err.kind() == std::io::ErrorKind::UnexpectedEof =>
            {
                break;
            }
            Err(err) => return Err(format!("decode packet for {}: {err}", path.display())),
        };

        let mut sample_buffer =
            SampleBuffer::<i16>::new(decoded.capacity() as u64, *decoded.spec());
        sample_buffer.copy_interleaved_ref(decoded);

        pcm_bytes.clear();
        pcm_bytes.reserve(sample_buffer.samples().len() * 2);
        for sample in sample_buffer.samples() {
            pcm_bytes.extend_from_slice(&sample.to_le_bytes());
        }
        stdin
            .write_all(&pcm_bytes)
            .map_err(|err| format!("write pcm to aplay for {}: {err}", path.display()))?;
    }

    drop(stdin);
    Ok(())
}

fn open_decoder(path: &Path) -> Result<(Box<dyn FormatReader>, u32, Box<dyn Decoder>), String> {
    let file = fs::File::open(path).map_err(|err| format!("open {}: {err}", path.display()))?;
    let media_source = MediaSourceStream::new(Box::new(file), Default::default());
    let mut hint = Hint::new();
    if let Some(extension) = path.extension().and_then(|value| value.to_str()) {
        hint.with_extension(extension);
    }

    let probed = get_probe()
        .format(
            &hint,
            media_source,
            &FormatOptions::default(),
            &MetadataOptions::default(),
        )
        .map_err(|err| format!("probe {}: {err}", path.display()))?;

    let format = probed.format;
    let (track_id, codec_params) = {
        let track = format
            .default_track()
            .ok_or_else(|| format!("no default audio track in {}", path.display()))?;
        (track.id, track.codec_params.clone())
    };
    let decoder = get_codecs()
        .make(&codec_params, &DecoderOptions::default())
        .map_err(|err| format!("create decoder for {}: {err}", path.display()))?;

    Ok((format, track_id, decoder))
}

fn detect_format(path: &Path) -> String {
    path.extension()
        .and_then(|value| value.to_str())
        .map(|value| value.to_ascii_lowercase())
        .unwrap_or_default()
}

fn empty_label(value: &str) -> &str {
    if value.is_empty() {
        "(unknown)"
    } else {
        value
    }
}

fn exit_status_label(status: &ExitStatus) -> String {
    if let Some(code) = status.code() {
        return format!("exit_code={code}");
    }

    if let Some(signal) = status.signal() {
        return format!("signal={signal}");
    }

    "unknown_exit".to_string()
}

fn normalize_snapshot(mut snapshot: QueueSnapshot) -> QueueSnapshot {
    if snapshot.tracks.is_empty() {
        snapshot.play_order.clear();
        snapshot.current_order_index = None;
        return snapshot;
    }

    let track_ids = snapshot
        .tracks
        .iter()
        .map(|entry| entry.queue_entry_id.clone())
        .collect::<Vec<_>>();

    if snapshot.play_order.is_empty() {
        snapshot.play_order = track_ids.clone();
    } else {
        snapshot.play_order.retain(|entry_id| {
            track_ids
                .iter()
                .any(|track_entry_id| track_entry_id == entry_id)
        });

        for track_id in &track_ids {
            if !snapshot
                .play_order
                .iter()
                .any(|entry_id| entry_id == track_id)
            {
                snapshot.play_order.push(track_id.clone());
            }
        }
    }

    if snapshot
        .current_order_index
        .is_some_and(|index| index >= snapshot.play_order.len())
    {
        snapshot.current_order_index = None;
    }

    snapshot
}

fn resolve_queue_entry(snapshot: &QueueSnapshot, index: Option<usize>) -> Option<QueueEntry> {
    let queue_entry_id = snapshot
        .play_order
        .get(index?)
        .map(|value| value.as_str())?;
    snapshot
        .tracks
        .iter()
        .find(|entry| entry.queue_entry_id == queue_entry_id)
        .cloned()
}

fn resolve_track_uid(snapshot: &QueueSnapshot, index: Option<usize>) -> Option<String> {
    resolve_queue_entry(snapshot, index).map(|entry| entry.track_uid)
}

fn next_queue_entry_id(snapshot: &QueueSnapshot) -> u64 {
    snapshot
        .tracks
        .iter()
        .filter_map(|entry| entry.queue_entry_id.strip_prefix('q'))
        .filter_map(|value| value.parse::<u64>().ok())
        .max()
        .unwrap_or(0)
        + 1
}

fn unix_timestamp_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn order_mode_label(mode: OrderMode) -> &'static str {
    match mode {
        OrderMode::Sequential => "sequential",
        OrderMode::Shuffle => "shuffle",
    }
}

fn repeat_mode_label(mode: RepeatMode) -> &'static str {
    match mode {
        RepeatMode::Off => "off",
        RepeatMode::One => "one",
        RepeatMode::All => "all",
    }
}

#[cfg(test)]
mod tests {
    use super::{
        load_history_log, load_runtime_state, normalize_snapshot, persist_history_log,
        persist_queue_snapshot, resolve_track_uid, RuntimeState, HISTORY_LIMIT,
    };
    use ipc_proto::parse_queue_snapshot_line;
    use media_model::{HistoryLog, OrderMode, QueueEntry, QueueSnapshot, RepeatMode};
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn restore_keeps_pointer_but_forces_stopped_state() {
        let path = temp_queue_path("restore_queue");
        let history_path = temp_queue_path("restore_history");
        let snapshot = shuffled_snapshot();
        persist_queue_snapshot(&path, &snapshot).unwrap();
        persist_history_log(&history_path, &HistoryLog::empty()).unwrap();

        let restored = load_runtime_state(&path, &history_path);

        assert_eq!(restored.playback_state.as_str(), "stopped");
        assert_eq!(restored.current_track.as_deref(), Some("track-b"));
        assert_eq!(restored.snapshot.current_order_index, Some(0));

        cleanup_temp_path(&path);
        cleanup_temp_path(&history_path);
    }

    #[test]
    fn stop_preserves_current_order_index() {
        let mut state = RuntimeState::new();
        state.apply_command(ipc_proto::PlaybackCommand::Play("track-a".to_string()));

        let outcome = state.apply_command(ipc_proto::PlaybackCommand::Stop);

        assert_eq!(state.playback_state.as_str(), "stopped");
        assert_eq!(state.snapshot.current_order_index, Some(0));
        assert_eq!(state.current_track.as_deref(), Some("track-a"));
        assert!(!outcome.persist_queue);
    }

    #[test]
    fn resolve_track_uses_play_order_not_track_vector_index() {
        let snapshot = shuffled_snapshot();

        assert_eq!(
            resolve_track_uid(&snapshot, Some(0)).as_deref(),
            Some("track-b")
        );
        assert_eq!(
            resolve_track_uid(&snapshot, Some(1)).as_deref(),
            Some("track-a")
        );
    }

    #[test]
    fn normalize_snapshot_appends_missing_track_ids() {
        let snapshot = QueueSnapshot {
            order_mode: OrderMode::Sequential,
            repeat_mode: RepeatMode::Off,
            current_order_index: Some(0),
            play_order: vec!["q2".to_string()],
            tracks: vec![queue_entry("q1", "track-a"), queue_entry("q2", "track-b")],
        };

        let normalized = normalize_snapshot(snapshot);

        assert_eq!(
            normalized.play_order,
            vec!["q2".to_string(), "q1".to_string()]
        );
    }

    #[test]
    fn play_appends_recent_history_entry() {
        let mut state = RuntimeState::new();

        state.apply_command(ipc_proto::PlaybackCommand::Play("track-a".to_string()));
        state.apply_command(ipc_proto::PlaybackCommand::Play("track-b".to_string()));

        assert_eq!(state.history_log.entries.len(), 2);
        assert_eq!(state.history_log.entries[0].track_uid, "track-b");
        assert_eq!(state.history_log.entries[1].track_uid, "track-a");
    }

    #[test]
    fn queue_append_selects_first_track_without_starting_playback() {
        let mut state = RuntimeState::new();

        let outcome = state.apply_command(ipc_proto::PlaybackCommand::QueueAppend(
            "track-a".to_string(),
        ));

        assert_eq!(state.playback_state.as_str(), "stopped");
        assert_eq!(state.current_track.as_deref(), Some("track-a"));
        assert_eq!(state.snapshot.current_order_index, Some(0));
        assert!(outcome.events.is_empty());
        assert!(outcome.persist_queue);
        assert!(!outcome.persist_history);
    }

    #[test]
    fn queue_insert_next_places_track_after_current_pointer() {
        let mut state = RuntimeState::new();
        state.apply_command(ipc_proto::PlaybackCommand::Play("track-a".to_string()));
        state.apply_command(ipc_proto::PlaybackCommand::QueueAppend(
            "track-c".to_string(),
        ));

        state.apply_command(ipc_proto::PlaybackCommand::QueueInsertNext(
            "track-b".to_string(),
        ));

        let play_order_tracks = state
            .snapshot
            .play_order
            .iter()
            .map(|queue_entry_id| {
                state
                    .snapshot
                    .tracks
                    .iter()
                    .find(|entry| &entry.queue_entry_id == queue_entry_id)
                    .map(|entry| entry.track_uid.clone())
                    .unwrap()
            })
            .collect::<Vec<_>>();
        assert_eq!(play_order_tracks, vec!["track-a", "track-b", "track-c"]);
        assert_eq!(state.current_track.as_deref(), Some("track-a"));
        assert_eq!(state.snapshot.current_order_index, Some(0));
    }

    #[test]
    fn queue_snapshot_command_exposes_order_and_current_marker() {
        let mut state = RuntimeState::new();
        state.apply_command(ipc_proto::PlaybackCommand::Play("track-a".to_string()));
        state.apply_command(ipc_proto::PlaybackCommand::QueueAppend(
            "track-c".to_string(),
        ));
        state.apply_command(ipc_proto::PlaybackCommand::QueueInsertNext(
            "track-b".to_string(),
        ));

        let outcome = state.apply_command(ipc_proto::PlaybackCommand::QueueSnapshot);
        let snapshot = parse_queue_snapshot_line(&outcome.response_line).unwrap();

        assert_eq!(snapshot.current_order_index, Some(0));
        assert_eq!(snapshot.entries.len(), 3);
~~~

## `services/rust/crates/playbackd/src/main.rs` (3/3)

- bytes: 84527
- segment: 3/3

~~~rust
        assert_eq!(snapshot.entries[0].track_uid, "track-a");
        assert_eq!(snapshot.entries[1].track_uid, "track-b");
        assert_eq!(snapshot.entries[2].track_uid, "track-c");
        assert!(snapshot.entries[0].is_current);
        assert!(!snapshot.entries[1].is_current);
    }

    #[test]
    fn queue_remove_before_current_shifts_pointer_without_switching_track() {
        let mut state = RuntimeState::new();
        state.apply_command(ipc_proto::PlaybackCommand::Play("track-a".to_string()));
        state.apply_command(ipc_proto::PlaybackCommand::QueueAppend(
            "track-b".to_string(),
        ));
        state.apply_command(ipc_proto::PlaybackCommand::QueueAppend(
            "track-c".to_string(),
        ));
        state.apply_command(ipc_proto::PlaybackCommand::Next);

        let remove_id = queue_entry_id_by_track(&state, "track-a");
        let outcome = state.apply_command(ipc_proto::PlaybackCommand::QueueRemove(remove_id));

        assert_eq!(state.snapshot.current_order_index, Some(0));
        assert_eq!(state.current_track.as_deref(), Some("track-b"));
        assert!(outcome.events.is_empty());
        assert!(outcome.persist_queue);
    }

    #[test]
    fn queue_remove_current_active_track_advances_and_records_history() {
        let mut state = RuntimeState::new();
        state.apply_command(ipc_proto::PlaybackCommand::Play("track-a".to_string()));
        state.apply_command(ipc_proto::PlaybackCommand::QueueAppend(
            "track-b".to_string(),
        ));

        let remove_id = queue_entry_id_by_track(&state, "track-a");
        let outcome = state.apply_command(ipc_proto::PlaybackCommand::QueueRemove(remove_id));

        assert_eq!(state.playback_state.as_str(), "quiet_active");
        assert_eq!(state.current_track.as_deref(), Some("track-b"));
        assert!(outcome.persist_history);
        assert_eq!(state.history_log.entries[0].track_uid, "track-b");
        assert_eq!(
            outcome.events,
            vec![ipc_proto::PlaybackEvent::TrackChanged {
                track_id: "track-b".to_string()
            }]
        );
    }

    #[test]
    fn queue_clear_from_active_state_stops_and_empties_queue() {
        let mut state = RuntimeState::new();
        state.apply_command(ipc_proto::PlaybackCommand::Play("track-a".to_string()));
        state.apply_command(ipc_proto::PlaybackCommand::QueueAppend(
            "track-b".to_string(),
        ));

        let outcome = state.apply_command(ipc_proto::PlaybackCommand::QueueClear);

        assert_eq!(state.playback_state.as_str(), "idle");
        assert!(state.snapshot.play_order.is_empty());
        assert!(state.snapshot.tracks.is_empty());
        assert_eq!(
            outcome.events,
            vec![ipc_proto::PlaybackEvent::PlaybackStopped {
                reason: "queue_cleared".to_string()
            }]
        );
    }

    #[test]
    fn queue_replace_stops_active_playback_and_selects_first_replacement() {
        let mut state = RuntimeState::new();
        state.apply_command(ipc_proto::PlaybackCommand::Play("track-a".to_string()));

        let outcome = state.apply_command(ipc_proto::PlaybackCommand::QueueReplace(vec![
            "track-x".to_string(),
            "track-y".to_string(),
        ]));

        assert_eq!(state.playback_state.as_str(), "stopped");
        assert_eq!(state.current_track.as_deref(), Some("track-x"));
        assert_eq!(state.snapshot.play_order.len(), 2);
        assert_eq!(
            outcome.events,
            vec![ipc_proto::PlaybackEvent::PlaybackStopped {
                reason: "queue_replaced".to_string()
            }]
        );
    }

    #[test]
    fn finished_track_advances_to_next_entry_and_records_history() {
        let mut state = RuntimeState::new();
        state.apply_command(ipc_proto::PlaybackCommand::Play("track-a".to_string()));
        state.apply_command(ipc_proto::PlaybackCommand::QueueAppend(
            "track-b".to_string(),
        ));

        let action = state.finish_track_output("track-a");

        assert_eq!(
            action,
            super::TrackFinishAction::Start {
                track_id: "track-b".to_string()
            }
        );
        assert_eq!(state.playback_state.as_str(), "quiet_active");
        assert_eq!(state.current_track.as_deref(), Some("track-b"));
        assert_eq!(state.snapshot.current_order_index, Some(1));
        assert_eq!(state.history_log.entries[0].track_uid, "track-b");
    }

    #[test]
    fn finished_track_stops_at_queue_end_when_repeat_is_off() {
        let mut state = RuntimeState::new();
        state.apply_command(ipc_proto::PlaybackCommand::Play("track-a".to_string()));

        let action = state.finish_track_output("track-a");

        assert_eq!(
            action,
            super::TrackFinishAction::Stop {
                reason: "track_finished".to_string()
            }
        );
        assert_eq!(state.playback_state.as_str(), "stopped");
        assert_eq!(state.current_track.as_deref(), Some("track-a"));
    }

    #[test]
    fn finished_track_wraps_to_queue_start_when_repeat_is_all() {
        let mut state = RuntimeState::new();
        state.apply_command(ipc_proto::PlaybackCommand::Play("track-a".to_string()));
        state.apply_command(ipc_proto::PlaybackCommand::QueueAppend(
            "track-b".to_string(),
        ));
        state.apply_command(ipc_proto::PlaybackCommand::Next);
        state.snapshot.repeat_mode = RepeatMode::All;

        let action = state.finish_track_output("track-b");

        assert_eq!(
            action,
            super::TrackFinishAction::Start {
                track_id: "track-a".to_string()
            }
        );
        assert_eq!(state.playback_state.as_str(), "quiet_active");
        assert_eq!(state.current_track.as_deref(), Some("track-a"));
        assert_eq!(state.snapshot.current_order_index, Some(0));
    }

    #[test]
    fn history_log_is_capped_to_latest_limit() {
        let mut state = RuntimeState::new();

        for index in 0..(HISTORY_LIMIT + 5) {
            state.apply_command(ipc_proto::PlaybackCommand::Play(format!("track-{index}")));
        }

        assert_eq!(state.history_log.entries.len(), HISTORY_LIMIT);
        assert_eq!(state.history_log.entries[0].track_uid, "track-104");
        assert_eq!(
            state.history_log.entries[HISTORY_LIMIT - 1].track_uid,
            "track-5"
        );
    }

    #[test]
    fn persisted_history_log_round_trips() {
        let path = temp_queue_path("history_round_trip");
        let mut state = RuntimeState::new();
        state.apply_command(ipc_proto::PlaybackCommand::Play("track-a".to_string()));
        state.apply_command(ipc_proto::PlaybackCommand::Play("track-b".to_string()));

        persist_history_log(&path, &state.history_log).unwrap();
        let restored = load_history_log(&path).unwrap();

        assert_eq!(restored.entries.len(), 2);
        assert_eq!(restored.entries[0].track_uid, "track-b");
        assert_eq!(restored.entries[1].track_uid, "track-a");

        cleanup_temp_path(&path);
    }

    fn shuffled_snapshot() -> QueueSnapshot {
        QueueSnapshot {
            order_mode: OrderMode::Shuffle,
            repeat_mode: RepeatMode::All,
            current_order_index: Some(0),
            play_order: vec!["q2".to_string(), "q1".to_string()],
            tracks: vec![queue_entry("q1", "track-a"), queue_entry("q2", "track-b")],
        }
    }

    fn queue_entry(queue_entry_id: &str, track_uid: &str) -> QueueEntry {
        QueueEntry {
            queue_entry_id: queue_entry_id.to_string(),
            track_uid: track_uid.to_string(),
            volume_uuid: "vol-1".to_string(),
            relative_path: track_uid.to_string(),
            title: Some(track_uid.to_string()),
            duration_ms: None,
        }
    }

    fn queue_entry_id_by_track(state: &RuntimeState, track_uid: &str) -> String {
        state
            .snapshot
            .tracks
            .iter()
            .find(|entry| entry.track_uid == track_uid)
            .map(|entry| entry.queue_entry_id.clone())
            .unwrap()
    }

    fn temp_queue_path(label: &str) -> PathBuf {
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        std::env::temp_dir().join(format!("lumelo-{label}-{suffix}.json"))
    }

    fn cleanup_temp_path(path: &PathBuf) {
        let _ = fs::remove_file(path);
    }
}
~~~

## `services/rust/crates/sessiond/Cargo.toml`

- bytes: 182
- segment: 1/1

~~~toml
[package]
name = "sessiond"
version.workspace = true
edition.workspace = true
license.workspace = true
authors.workspace = true

[dependencies]
ipc-proto = { path = "../ipc-proto" }
~~~

## `services/rust/crates/sessiond/src/main.rs`

- bytes: 6105
- segment: 1/1

~~~rust
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
~~~

## `tests/README.md`

- bytes: 116
- segment: 1/1

~~~md
# Tests

Use this directory for integration and system-level tests that span services or
rootfs packaging behavior.
~~~

