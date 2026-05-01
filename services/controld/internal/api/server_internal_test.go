package api

import (
	"net/url"
	"reflect"
	"strings"
	"testing"

	"github.com/lumelo/controld/internal/libraryclient"
	"github.com/lumelo/controld/internal/playbackclient"
)

func TestTrackIDsFromPlaybackContextStartsAtSelectedTrack(t *testing.T) {
	snapshot := libraryclient.Snapshot{
		Tracks: []libraryclient.TrackSummary{
			{TrackUID: "track-a"},
			{TrackUID: "track-b"},
			{TrackUID: "track-c"},
		},
	}

	trackIDs, err := trackIDsFromPlaybackContext(snapshot, "track-b")
	if err != nil {
		t.Fatalf("trackIDsFromPlaybackContext: %v", err)
	}

	expected := []string{"track-b", "track-c"}
	if !reflect.DeepEqual(trackIDs, expected) {
		t.Fatalf("unexpected track ids: got %v want %v", trackIDs, expected)
	}
}

func TestTrackIDsFromPlaybackContextRejectsAbsoluteTrackUIDs(t *testing.T) {
	snapshot := libraryclient.Snapshot{
		Tracks: []libraryclient.TrackSummary{
			{TrackUID: "track-a"},
			{TrackUID: "/tmp/manual.wav"},
		},
	}

	_, err := trackIDsFromPlaybackContext(snapshot, "track-a")
	if err == nil || !strings.Contains(err.Error(), "absolute_path_playback_forbidden") {
		t.Fatalf("expected absolute path rejection, got %v", err)
	}
}

func TestLibraryQueryFromValuesReadsAlbumFilter(t *testing.T) {
	values := url.Values{}
	values.Set("album_uid", "album-123")
	values.Set("volume_uuid", "vol-ignored")
	values.Set("directory", "OST")

	query := libraryQueryFromValues(values)
	if query.AlbumUID != "album-123" {
		t.Fatalf("unexpected query: %+v", query)
	}
	if query.DirectoryVolumeUUID != "vol-ignored" || query.DirectoryPath != "OST" {
		t.Fatalf("unexpected directory query fields: %+v", query)
	}
}

func TestSelectedAlbumTitleReturnsMatch(t *testing.T) {
	snapshot := libraryclient.Snapshot{
		Albums: []libraryclient.AlbumSummary{
			{AlbumUID: "album-123", Title: "Album A"},
			{AlbumUID: "album-456", Title: "Album B"},
		},
	}

	if got := selectedAlbumTitle(snapshot, "album-456"); got != "Album B" {
		t.Fatalf("unexpected selected album title: %s", got)
	}
}

func TestBuildLibraryAlbumViewsAddsSyntheticUncategorizedBucket(t *testing.T) {
	snapshot := libraryclient.Snapshot{
		Albums: []libraryclient.AlbumSummary{
			{AlbumUID: "album-123", Title: "Album A", AlbumArtist: "Artist A", SourceMode: "tag", TrackCount: 2},
			{AlbumUID: "album-456", Title: "Loose Track", AlbumArtist: "Unknown Artist", SourceMode: "directory_fallback", TrackCount: 1},
		},
	}

	views := buildLibraryAlbumViews(snapshot, libraryclient.UncategorizedAlbumUID)
	if len(views) != 2 {
		t.Fatalf("unexpected album view count: %+v", views)
	}
	last := views[len(views)-1]
	if last.AlbumUID != libraryclient.UncategorizedAlbumUID || !last.IsSynthetic || !last.IsSelected {
		t.Fatalf("unexpected uncategorized view: %+v", last)
	}
	if last.TrackCount != 1 {
		t.Fatalf("unexpected uncategorized track count: %+v", last)
	}
}

func TestBuildHistoryEntryViewsOnlyMarksCurrentWhenPlaybackActive(t *testing.T) {
	title := "Track A"
	snapshot := playbackclient.HistorySnapshot{
		Available: true,
		Entries: []playbackclient.HistoryEntry{
			{TrackUID: "track-a", Title: &title},
		},
	}

	stoppedViews := buildHistoryEntryViews(snapshot, playbackclient.Status{
		Available:    true,
		State:        "stopped",
		CurrentTrack: "track-a",
		QueueEntries: 1,
		OrderMode:    "shuffle",
		RepeatMode:   "off",
		LastCommand:  "status",
	}, libraryclient.Snapshot{})
	if len(stoppedViews) != 1 || stoppedViews[0].IsCurrent {
		t.Fatalf("stopped playback must not mark history current: %+v", stoppedViews)
	}

	activeViews := buildHistoryEntryViews(snapshot, playbackclient.Status{
		Available:    true,
		State:        "quiet_active",
		CurrentTrack: "track-a",
	}, libraryclient.Snapshot{})
	if len(activeViews) != 1 || !activeViews[0].IsCurrent {
		t.Fatalf("active playback should mark history current: %+v", activeViews)
	}
}

func TestBuildHistoryEntryViewsOnlyMarksNewestMatchingCurrent(t *testing.T) {
	views := buildHistoryEntryViews(
		playbackclient.HistorySnapshot{
			Available: true,
			Entries: []playbackclient.HistoryEntry{
				{TrackUID: "track-a"},
				{TrackUID: "track-b"},
				{TrackUID: "track-a"},
			},
		},
		playbackclient.Status{Available: true, State: "quiet_active", CurrentTrack: "track-a"},
		libraryclient.Snapshot{},
	)

	if len(views) != 3 {
		t.Fatalf("unexpected history view count: %+v", views)
	}
	if !views[0].IsCurrent {
		t.Fatalf("newest matching history entry should be current: %+v", views)
	}
	if views[2].IsCurrent {
		t.Fatalf("older duplicate history entry must not be current: %+v", views)
	}
}

func TestBuildHistoryEntryViewsEnrichesTitleFromLibrary(t *testing.T) {
	title := "track-a"
	views := buildHistoryEntryViews(
		playbackclient.HistorySnapshot{
			Available: true,
			Entries: []playbackclient.HistoryEntry{
				{TrackUID: "track-a", RelativePath: "track-a", Title: &title},
			},
		},
		playbackclient.Status{Available: true, State: "stopped", CurrentTrack: "track-a"},
		libraryclient.Snapshot{
			Tracks: []libraryclient.TrackSummary{
				{TrackUID: "track-a", Title: "Real Title", RelativePath: "Album/01 Real Title.flac"},
			},
		},
	)

	if len(views) != 1 || views[0].Title != "Real Title" || views[0].RelativePath != "Album/01 Real Title.flac" {
		t.Fatalf("expected library-enriched history view: %+v", views)
	}
}

func TestBuildHistoryEntryViewsKeepsRecentFirstOrder(t *testing.T) {
	views := buildHistoryEntryViews(
		playbackclient.HistorySnapshot{
			Available: true,
			Entries: []playbackclient.HistoryEntry{
				{TrackUID: "recent-track"},
				{TrackUID: "older-track"},
			},
		},
		playbackclient.Status{Available: true, State: "stopped"},
		libraryclient.Snapshot{},
	)

	if len(views) != 2 {
		t.Fatalf("unexpected history view count: %+v", views)
	}
	if views[0].TrackUID != "recent-track" || views[0].DisplayIndex != "01" {
		t.Fatalf("expected most recent track first: %+v", views)
	}
	if views[1].TrackUID != "older-track" || views[1].DisplayIndex != "02" {
		t.Fatalf("expected older track second: %+v", views)
	}
}

func TestTransportPrimaryActionFollowsPlaybackState(t *testing.T) {
	for _, state := range []string{"stopped", "paused", ""} {
		if action := transportPrimaryAction(state); action != "play" {
			t.Fatalf("expected play action for %q, got %q", state, action)
		}
		if label := transportPrimaryLabel(state); label != "播放" {
			t.Fatalf("expected play label for %q, got %q", state, label)
		}
	}

	for _, state := range []string{"playing", "quiet_active", "pre_quiet"} {
		if action := transportPrimaryAction(state); action != "pause" {
			t.Fatalf("expected pause action for %q, got %q", state, action)
		}
		if label := transportPrimaryLabel(state); label != "暂停" {
			t.Fatalf("expected pause label for %q, got %q", state, label)
		}
	}
}
