package playbackclient

import (
	"context"
	"net"
	"os"
	"path/filepath"
	"reflect"
	"testing"
	"time"
)

func TestStatusParsesUnixSocketResponse(t *testing.T) {
	socketPath := shortSocketPath(t)
	listener, err := net.Listen("unix", socketPath)
	if err != nil {
		t.Fatalf("listen unix socket: %v", err)
	}
	defer listener.Close()

	done := make(chan struct{})
	go func() {
		defer close(done)
		conn, err := listener.Accept()
		if err != nil {
			return
		}
		defer conn.Close()

		buf := make([]byte, 128)
		_, _ = conn.Read(buf)
		_, _ = conn.Write([]byte("OK\tkind=status\tstate=quiet_active\torder_mode=sequential\trepeat_mode=off\tcurrent_track=demo track\tlast_command=play:demo track\tqueue_entries=2\n"))
	}()

	client := New(socketPath, "")
	ctx, cancel := context.WithTimeout(context.Background(), 2*time.Second)
	defer cancel()

	status := client.Status(ctx)
	if !status.Available {
		t.Fatalf("expected status to be available, got error: %s", status.Error)
	}
	if status.State != "quiet_active" {
		t.Fatalf("unexpected state: %s", status.State)
	}
	if status.CurrentTrack != "demo track" {
		t.Fatalf("unexpected current track: %s", status.CurrentTrack)
	}
	if status.QueueEntries != 2 {
		t.Fatalf("unexpected queue entry count: %d", status.QueueEntries)
	}

	<-done
}

func TestExecuteFormatsPlayHistoryCommand(t *testing.T) {
	socketPath := shortSocketPath(t)
	listener, err := net.Listen("unix", socketPath)
	if err != nil {
		t.Fatalf("listen unix socket: %v", err)
	}
	defer listener.Close()

	done := make(chan struct{})
	go func() {
		defer close(done)
		conn, err := listener.Accept()
		if err != nil {
			return
		}
		defer conn.Close()

		buf := make([]byte, 256)
		n, _ := conn.Read(buf)
		if string(buf[:n]) != "PLAY_HISTORY side a track 01\n" {
			t.Errorf("unexpected command line: %q", string(buf[:n]))
			return
		}

		_, _ = conn.Write([]byte("OK\tkind=ack\taction=play_history\tstate=quiet_active\tcurrent_track=side a track 01\n"))
	}()

	client := New(socketPath, "")
	ctx, cancel := context.WithTimeout(context.Background(), 2*time.Second)
	defer cancel()

	message, err := client.Execute(ctx, "play_history", "side a track 01")
	if err != nil {
		t.Fatalf("execute play_history: %v", err)
	}
	if message != "PLAY_HISTORY -> state=quiet_active current=side a track 01" {
		t.Fatalf("unexpected execute message: %s", message)
	}

	<-done
}

func TestExecuteFormatsQueueReplaceCommand(t *testing.T) {
	socketPath := shortSocketPath(t)
	listener, err := net.Listen("unix", socketPath)
	if err != nil {
		t.Fatalf("listen unix socket: %v", err)
	}
	defer listener.Close()

	done := make(chan struct{})
	go func() {
		defer close(done)
		conn, err := listener.Accept()
		if err != nil {
			return
		}
		defer conn.Close()

		buf := make([]byte, 256)
		n, _ := conn.Read(buf)
		if string(buf[:n]) != "QUEUE_REPLACE [\"side a 01\",\"side b 02\"]\n" {
			t.Errorf("unexpected command line: %q", string(buf[:n]))
			return
		}

		_, _ = conn.Write([]byte("OK\tkind=ack\taction=queue_replace\tstate=stopped\tcurrent_track=side a 01\n"))
	}()

	client := New(socketPath, "")
	ctx, cancel := context.WithTimeout(context.Background(), 2*time.Second)
	defer cancel()

	message, err := client.Execute(ctx, "queue_replace", "[\"side a 01\",\"side b 02\"]")
	if err != nil {
		t.Fatalf("execute queue_replace: %v", err)
	}
	if message != "QUEUE_REPLACE -> state=stopped current=side a 01" {
		t.Fatalf("unexpected execute message: %s", message)
	}

	<-done
}

func TestQueueSnapshotParsesSnapshotPayload(t *testing.T) {
	socketPath := shortSocketPath(t)
	listener, err := net.Listen("unix", socketPath)
	if err != nil {
		t.Fatalf("listen unix socket: %v", err)
	}
	defer listener.Close()

	done := make(chan struct{})
	go func() {
		defer close(done)
		conn, err := listener.Accept()
		if err != nil {
			return
		}
		defer conn.Close()

		buf := make([]byte, 256)
		n, _ := conn.Read(buf)
		if string(buf[:n]) != "QUEUE_SNAPSHOT\n" {
			t.Errorf("unexpected command line: %q", string(buf[:n]))
			return
		}

		_, _ = conn.Write([]byte("OK\tkind=queue_snapshot\tpayload={\"order_mode\":\"sequential\",\"repeat_mode\":\"off\",\"current_order_index\":0,\"entries\":[{\"order_index\":0,\"queue_entry_id\":\"q1\",\"track_uid\":\"track-a\",\"volume_uuid\":\"manual\",\"relative_path\":\"track-a\",\"title\":\"track-a\",\"duration_ms\":null,\"is_current\":true}]}\n"))
	}()

	client := New(socketPath, "")
	ctx, cancel := context.WithTimeout(context.Background(), 2*time.Second)
	defer cancel()

	snapshot := client.QueueSnapshot(ctx)
	if !snapshot.Available {
		t.Fatalf("expected queue snapshot to be available, got error: %s", snapshot.Error)
	}
	if snapshot.CurrentOrderIndex == nil || *snapshot.CurrentOrderIndex != 0 {
		t.Fatalf("unexpected current order index: %#v", snapshot.CurrentOrderIndex)
	}
	if len(snapshot.Entries) != 1 {
		t.Fatalf("unexpected entry count: %d", len(snapshot.Entries))
	}
	if snapshot.Entries[0].QueueEntryID != "q1" {
		t.Fatalf("unexpected queue entry id: %s", snapshot.Entries[0].QueueEntryID)
	}
	if !snapshot.Entries[0].IsCurrent {
		t.Fatalf("expected entry to be current")
	}

	<-done
}

func TestExecuteFormatsQueueSnapshotResponse(t *testing.T) {
	socketPath := shortSocketPath(t)
	listener, err := net.Listen("unix", socketPath)
	if err != nil {
		t.Fatalf("listen unix socket: %v", err)
	}
	defer listener.Close()

	done := make(chan struct{})
	go func() {
		defer close(done)
		conn, err := listener.Accept()
		if err != nil {
			return
		}
		defer conn.Close()

		buf := make([]byte, 256)
		n, _ := conn.Read(buf)
		if string(buf[:n]) != "QUEUE_SNAPSHOT\n" {
			t.Errorf("unexpected command line: %q", string(buf[:n]))
			return
		}

		_, _ = conn.Write([]byte("OK\tkind=queue_snapshot\tpayload={\"order_mode\":\"sequential\",\"repeat_mode\":\"off\",\"current_order_index\":1,\"entries\":[{\"order_index\":0,\"queue_entry_id\":\"q1\",\"track_uid\":\"track-a\",\"volume_uuid\":\"manual\",\"relative_path\":\"track-a\",\"title\":\"track-a\",\"duration_ms\":null,\"is_current\":false},{\"order_index\":1,\"queue_entry_id\":\"q2\",\"track_uid\":\"track-b\",\"volume_uuid\":\"manual\",\"relative_path\":\"track-b\",\"title\":\"track-b\",\"duration_ms\":null,\"is_current\":true}]}\n"))
	}()

	client := New(socketPath, "")
	ctx, cancel := context.WithTimeout(context.Background(), 2*time.Second)
	defer cancel()

	message, err := client.Execute(ctx, "queue_snapshot", "")
	if err != nil {
		t.Fatalf("execute queue_snapshot: %v", err)
	}
	if message != "QUEUE_SNAPSHOT -> entries=2 current_index=1" {
		t.Fatalf("unexpected execute message: %s", message)
	}

	<-done
}

func TestSubscribeEventsParsesPlaybackEventStream(t *testing.T) {
	socketPath := shortSocketPath(t)
	listener, err := net.Listen("unix", socketPath)
	if err != nil {
		t.Fatalf("listen unix socket: %v", err)
	}
	defer listener.Close()

	done := make(chan struct{})
	go func() {
		defer close(done)
		conn, err := listener.Accept()
		if err != nil {
			return
		}
		defer conn.Close()

		_, _ = conn.Write([]byte("EVENT\tname=PLAYBACK_STARTED\ttrack_id=demo-track-001\n"))
		_, _ = conn.Write([]byte("EVENT\tname=TRACK_CHANGED\ttrack_id=demo-track-002\n"))
	}()

	client := New("", socketPath)
	ctx, cancel := context.WithTimeout(context.Background(), 2*time.Second)
	defer cancel()

	var events []Event
	err = client.SubscribeEvents(ctx, func(event Event) error {
		events = append(events, event)
		if len(events) == 2 {
			cancel()
		}
		return nil
	})
	if err != nil && err != context.Canceled {
		t.Fatalf("subscribe events: %v", err)
	}

	want := []Event{
		{Name: "PLAYBACK_STARTED", TrackID: "demo-track-001"},
		{Name: "TRACK_CHANGED", TrackID: "demo-track-002"},
	}
	if !reflect.DeepEqual(events, want) {
		t.Fatalf("unexpected events: %#v", events)
	}

	<-done
}

func shortSocketPath(t *testing.T) string {
	t.Helper()

	dir, err := os.MkdirTemp("/tmp", "npct4-uds-")
	if err != nil {
		t.Fatalf("create short temp dir: %v", err)
	}
	t.Cleanup(func() {
		_ = os.RemoveAll(dir)
	})

	return filepath.Join(dir, "playback.sock")
}
