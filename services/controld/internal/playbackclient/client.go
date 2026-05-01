package playbackclient

import (
	"bufio"
	"context"
	"encoding/json"
	"fmt"
	"io"
	"net"
	"strconv"
	"strings"
	"time"
)

type Client struct {
	CommandSocket string
	EventSocket   string
}

type Status struct {
	Available    bool
	State        string
	OrderMode    string
	RepeatMode   string
	CurrentTrack string
	LastCommand  string
	QueueEntries int
	Raw          string
	Error        string
}

type QueueSnapshot struct {
	Available         bool
	OrderMode         string
	RepeatMode        string
	CurrentOrderIndex *int
	Entries           []QueueEntry
	Raw               string
	Error             string
}

type HistorySnapshot struct {
	Available bool
	Entries   []HistoryEntry
	Raw       string
	Error     string
}

type QueueEntry struct {
	OrderIndex   int     `json:"order_index"`
	QueueEntryID string  `json:"queue_entry_id"`
	TrackUID     string  `json:"track_uid"`
	VolumeUUID   string  `json:"volume_uuid"`
	RelativePath string  `json:"relative_path"`
	Title        *string `json:"title"`
	DurationMS   *uint64 `json:"duration_ms"`
	IsCurrent    bool    `json:"is_current"`
}

type HistoryEntry struct {
	PlayedAt     uint64  `json:"played_at"`
	TrackUID     string  `json:"track_uid"`
	VolumeUUID   string  `json:"volume_uuid"`
	RelativePath string  `json:"relative_path"`
	Title        *string `json:"title"`
	DurationMS   *uint64 `json:"duration_ms"`
}

type Event struct {
	Name        string `json:"name"`
	TrackID     string `json:"track_id,omitempty"`
	Reason      string `json:"reason,omitempty"`
	Class       string `json:"class,omitempty"`
	Recoverable bool   `json:"recoverable,omitempty"`
	KeepQuiet   bool   `json:"keep_quiet,omitempty"`
}

func New(commandSocket, eventSocket string) *Client {
	return &Client{
		CommandSocket: commandSocket,
		EventSocket:   eventSocket,
	}
}

func (c *Client) Status(ctx context.Context) Status {
	line, err := c.request(ctx, "STATUS")
	if err != nil {
		return Status{Error: err.Error()}
	}

	status, err := parseStatusResponse(line)
	if err != nil {
		return Status{Raw: line, Error: err.Error()}
	}

	status.Available = true
	status.Raw = line
	return status
}

func (c *Client) QueueSnapshot(ctx context.Context) QueueSnapshot {
	line, err := c.request(ctx, "QUEUE_SNAPSHOT")
	if err != nil {
		return QueueSnapshot{Error: err.Error()}
	}

	snapshot, err := parseQueueSnapshotResponse(line)
	if err != nil {
		return QueueSnapshot{Raw: line, Error: err.Error()}
	}

	snapshot.Available = true
	snapshot.Raw = line
	return snapshot
}

func (c *Client) HistorySnapshot(ctx context.Context) HistorySnapshot {
	line, err := c.request(ctx, "HISTORY_SNAPSHOT")
	if err != nil {
		return HistorySnapshot{Error: err.Error()}
	}

	snapshot, err := parseHistorySnapshotResponse(line)
	if err != nil {
		return HistorySnapshot{Raw: line, Error: err.Error()}
	}

	snapshot.Available = true
	snapshot.Raw = line
	return snapshot
}

func (c *Client) Execute(ctx context.Context, action, trackID string) (string, error) {
	line, err := commandLine(action, trackID)
	if err != nil {
		return "", err
	}

	response, err := c.request(ctx, line)
	if err != nil {
		return "", err
	}

	kind, fields, err := parseResponse(response)
	if err != nil {
		return "", err
	}
	if kind == "ERR" {
		return "", fmt.Errorf("%s: %s", fields["code"], fields["message"])
	}

	if fields["kind"] == "status" {
		status, err := parseStatusResponse(response)
		if err != nil {
			return "", err
		}

		current := placeholder(status.CurrentTrack)
		return fmt.Sprintf("STATUS -> state=%s current=%s", status.State, current), nil
	}
	if fields["kind"] == "queue_snapshot" {
		snapshot, err := parseQueueSnapshotResponse(response)
		if err != nil {
			return "", err
		}

		return fmt.Sprintf("QUEUE_SNAPSHOT -> entries=%d current_index=%s", len(snapshot.Entries), pointerLabel(snapshot.CurrentOrderIndex)), nil
	}
	if fields["kind"] == "history_snapshot" {
		snapshot, err := parseHistorySnapshotResponse(response)
		if err != nil {
			return "", err
		}

		return fmt.Sprintf("HISTORY_SNAPSHOT -> entries=%d", len(snapshot.Entries)), nil
	}

	actionName := strings.ToUpper(fields["action"])
	state := fields["state"]
	currentTrack := placeholder(fields["current_track"])
	return fmt.Sprintf("%s -> state=%s current=%s", actionName, state, currentTrack), nil
}

func (c *Client) PlayQueue(ctx context.Context, trackIDs []string) (string, error) {
	if len(trackIDs) == 0 {
		return "", fmt.Errorf("at least one track id is required for QUEUE_PLAY")
	}

	payload, err := json.Marshal(trackIDs)
	if err != nil {
		return "", fmt.Errorf("encode track context: %w", err)
	}

	return c.Execute(ctx, "queue_play", string(payload))
}

func (c *Client) SubscribeEvents(ctx context.Context, handler func(Event) error) error {
	dialer := &net.Dialer{Timeout: 2 * time.Second}
	conn, err := dialer.DialContext(ctx, "unix", c.EventSocket)
	if err != nil {
		return fmt.Errorf("dial playback event socket: %w", err)
	}
	defer conn.Close()

	reader := bufio.NewReader(conn)
	for {
		if deadline, ok := ctx.Deadline(); ok {
			_ = conn.SetReadDeadline(deadline)
		} else {
			_ = conn.SetReadDeadline(time.Now().Add(30 * time.Second))
		}

		line, err := reader.ReadString('\n')
		if err != nil {
			if ctx.Err() != nil {
				return ctx.Err()
			}
			if netErr, ok := err.(net.Error); ok && netErr.Timeout() {
				continue
			}
			if err == io.EOF {
				return nil
			}
			return fmt.Errorf("read playback event: %w", err)
		}

		event, err := parseEventLine(line)
		if err != nil {
			return err
		}
		if err := handler(event); err != nil {
			return err
		}
	}
}

func (c *Client) request(ctx context.Context, line string) (string, error) {
	dialer := &net.Dialer{Timeout: 2 * time.Second}
	conn, err := dialer.DialContext(ctx, "unix", c.CommandSocket)
	if err != nil {
		return "", fmt.Errorf("dial playback socket: %w", err)
	}
	defer conn.Close()

	if deadline, ok := ctx.Deadline(); ok {
		_ = conn.SetDeadline(deadline)
	} else {
		_ = conn.SetDeadline(time.Now().Add(2 * time.Second))
	}

	if _, err := io.WriteString(conn, line+"\n"); err != nil {
		return "", fmt.Errorf("write playback command: %w", err)
	}

	response, err := bufio.NewReader(conn).ReadString('\n')
	if err != nil && err != io.EOF {
		return "", fmt.Errorf("read playback response: %w", err)
	}

	return strings.TrimRight(response, "\r\n"), nil
}

func commandLine(action, trackID string) (string, error) {
	switch strings.ToLower(strings.TrimSpace(action)) {
	case "ping":
		return "PING", nil
	case "status":
		return "STATUS", nil
	case "queue_snapshot":
		return "QUEUE_SNAPSHOT", nil
	case "history_snapshot":
		return "HISTORY_SNAPSHOT", nil
	case "play":
		trackID = strings.TrimSpace(trackID)
		if trackID == "" {
			return "", fmt.Errorf("track id is required for PLAY")
		}
		return "PLAY " + trackID, nil
	case "play_history":
		trackID = strings.TrimSpace(trackID)
		if trackID == "" {
			return "", fmt.Errorf("track id is required for PLAY_HISTORY")
		}
		return "PLAY_HISTORY " + trackID, nil
	case "queue_append":
		trackID = strings.TrimSpace(trackID)
		if trackID == "" {
			return "", fmt.Errorf("track id is required for QUEUE_APPEND")
		}
		return "QUEUE_APPEND " + trackID, nil
	case "queue_insert_next":
		trackID = strings.TrimSpace(trackID)
		if trackID == "" {
			return "", fmt.Errorf("track id is required for QUEUE_INSERT_NEXT")
		}
		return "QUEUE_INSERT_NEXT " + trackID, nil
	case "queue_remove":
		trackID = strings.TrimSpace(trackID)
		if trackID == "" {
			return "", fmt.Errorf("queue entry id is required for QUEUE_REMOVE")
		}
		return "QUEUE_REMOVE " + trackID, nil
	case "queue_clear":
		return "QUEUE_CLEAR", nil
	case "queue_replace":
		trackID = strings.TrimSpace(trackID)
		if trackID == "" {
			return "", fmt.Errorf("JSON track list is required for QUEUE_REPLACE")
		}
		return "QUEUE_REPLACE " + trackID, nil
	case "queue_play":
		trackID = strings.TrimSpace(trackID)
		if trackID == "" {
			return "", fmt.Errorf("JSON track list is required for QUEUE_PLAY")
		}
		return "QUEUE_PLAY " + trackID, nil
	case "pause":
		return "PAUSE", nil
	case "stop":
		return "STOP", nil
	case "next":
		return "NEXT", nil
	case "prev":
		return "PREV", nil
	default:
		return "", fmt.Errorf("unsupported action: %s", action)
	}
}

func parseStatusResponse(line string) (Status, error) {
	kind, fields, err := parseResponse(line)
	if err != nil {
		return Status{}, err
	}
	if kind != "OK" {
		return Status{}, fmt.Errorf("unexpected non-OK playback response")
	}
	if fields["kind"] != "status" {
		return Status{}, fmt.Errorf("unexpected playback response kind: %s", fields["kind"])
	}

	queueEntries, err := strconv.Atoi(fields["queue_entries"])
	if err != nil {
		return Status{}, fmt.Errorf("invalid queue_entries value: %w", err)
	}

	return Status{
		State:        fields["state"],
		OrderMode:    fields["order_mode"],
		RepeatMode:   fields["repeat_mode"],
		CurrentTrack: blankIfPlaceholder(fields["current_track"]),
		LastCommand:  blankIfPlaceholder(fields["last_command"]),
		QueueEntries: queueEntries,
	}, nil
}

func parseQueueSnapshotResponse(line string) (QueueSnapshot, error) {
	kind, fields, err := parseResponse(line)
	if err != nil {
		return QueueSnapshot{}, err
	}
	if kind != "OK" {
		return QueueSnapshot{}, fmt.Errorf("unexpected non-OK playback response")
	}
	if fields["kind"] != "queue_snapshot" {
		return QueueSnapshot{}, fmt.Errorf("unexpected playback response kind: %s", fields["kind"])
	}

	var payload struct {
		OrderMode         string       `json:"order_mode"`
		RepeatMode        string       `json:"repeat_mode"`
		CurrentOrderIndex *int         `json:"current_order_index"`
		Entries           []QueueEntry `json:"entries"`
	}
	if err := json.Unmarshal([]byte(fields["payload"]), &payload); err != nil {
		return QueueSnapshot{}, fmt.Errorf("invalid queue snapshot payload: %w", err)
	}

	return QueueSnapshot{
		OrderMode:         payload.OrderMode,
		RepeatMode:        payload.RepeatMode,
		CurrentOrderIndex: payload.CurrentOrderIndex,
		Entries:           payload.Entries,
	}, nil
}

func parseHistorySnapshotResponse(line string) (HistorySnapshot, error) {
	kind, fields, err := parseResponse(line)
	if err != nil {
		return HistorySnapshot{}, err
	}
	if kind != "OK" {
		return HistorySnapshot{}, fmt.Errorf("unexpected non-OK playback response")
	}
	if fields["kind"] != "history_snapshot" {
		return HistorySnapshot{}, fmt.Errorf("unexpected playback response kind: %s", fields["kind"])
	}

	var payload struct {
		Entries []HistoryEntry `json:"entries"`
	}
	if err := json.Unmarshal([]byte(fields["payload"]), &payload); err != nil {
		return HistorySnapshot{}, fmt.Errorf("invalid history snapshot payload: %w", err)
	}

	return HistorySnapshot{Entries: payload.Entries}, nil
}

func parseEventLine(line string) (Event, error) {
	trimmed := strings.TrimSpace(line)
	if trimmed == "" {
		return Event{}, fmt.Errorf("playback event is empty")
	}

	parts := strings.Split(trimmed, "\t")
	if parts[0] != "EVENT" {
		return Event{}, fmt.Errorf("unsupported playback event prefix: %s", parts[0])
	}

	fields := make(map[string]string, len(parts)-1)
	for _, part := range parts[1:] {
		key, value, ok := strings.Cut(part, "=")
		if !ok {
			return Event{}, fmt.Errorf("malformed playback event field: %s", part)
		}
		fields[key] = value
	}

	event := Event{Name: fields["name"]}
	switch event.Name {
	case "PLAY_REQUEST_ACCEPTED", "PLAYBACK_STARTED", "TRACK_CHANGED":
		event.TrackID = fields["track_id"]
	case "PLAYBACK_STOPPED":
		event.Reason = fields["reason"]
	case "PLAYBACK_FAILED":
		event.Reason = fields["reason"]
		event.Class = fields["class"]
		event.Recoverable = fields["recoverable"] == "true"
		event.KeepQuiet = fields["keep_quiet"] == "true"
	case "PLAYBACK_PAUSED", "PLAYBACK_RESUMED":
	default:
		return Event{}, fmt.Errorf("unsupported playback event name: %s", event.Name)
	}

	return event, nil
}

func parseResponse(line string) (string, map[string]string, error) {
	trimmed := strings.TrimSpace(line)
	if trimmed == "" {
		return "", nil, fmt.Errorf("playback response is empty")
	}

	parts := strings.Split(trimmed, "\t")
	kind := parts[0]
	if kind != "OK" && kind != "ERR" {
		return "", nil, fmt.Errorf("unsupported playback response prefix: %s", kind)
	}

	fields := make(map[string]string, len(parts)-1)
	for _, part := range parts[1:] {
		key, value, ok := strings.Cut(part, "=")
		if !ok {
			return "", nil, fmt.Errorf("malformed playback field: %s", part)
		}
		fields[key] = value
	}

	return kind, fields, nil
}

func blankIfPlaceholder(value string) string {
	if value == "-" {
		return ""
	}

	return value
}

func placeholder(value string) string {
	if strings.TrimSpace(value) == "" {
		return "-"
	}

	return value
}

func pointerLabel(value *int) string {
	if value == nil {
		return "-"
	}

	return strconv.Itoa(*value)
}
