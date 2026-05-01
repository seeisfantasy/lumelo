package main

import (
	"log"
	"net/http"
	"os"
	"path/filepath"

	"github.com/lumelo/controld/internal/api"
	"github.com/lumelo/controld/internal/audiodevice"
	"github.com/lumelo/controld/internal/auth"
	"github.com/lumelo/controld/internal/libraryclient"
	"github.com/lumelo/controld/internal/logclient"
	"github.com/lumelo/controld/internal/mediaimport"
	"github.com/lumelo/controld/internal/playbackclient"
	"github.com/lumelo/controld/internal/provisioningclient"
	"github.com/lumelo/controld/internal/settings"
	"github.com/lumelo/controld/internal/sshctl"
	"github.com/lumelo/controld/web"
)

func main() {
	configPath := getenvWithFallbacks([]string{"CONTROLD_CONFIG_PATH", "LUMELO_CONFIG_PATH"}, settings.Default().ConfigPath)
	cfg, err := settings.Load(configPath)
	if err != nil {
		log.Printf("load controld config %s: %v; using defaults", configPath, err)
		cfg = settings.Default()
		cfg.ConfigPath = configPath
	}
	runtimeDir := getenvWithFallbacks([]string{"LUMELO_RUNTIME_DIR", "PRODUCT_RUNTIME_DIR"}, "/run/lumelo")
	stateDir := getenvWithFallbacks([]string{"LUMELO_STATE_DIR", "PRODUCT_STATE_DIR"}, "/var/lib/lumelo")
	commandSocket := getenv("CONTROLD_PLAYBACK_CMD_SOCKET", filepath.Join(runtimeDir, "playback_cmd.sock"))
	eventSocket := getenv("CONTROLD_PLAYBACK_EVT_SOCKET", filepath.Join(runtimeDir, "playback_evt.sock"))
	libraryDBPath := getenvWithFallbacks(
		[]string{"CONTROLD_LIBRARY_DB_PATH", "LIBRARY_DB_PATH"},
		filepath.Join(stateDir, "library.db"),
	)
	artworkCacheRoot := getenv("CONTROLD_ARTWORK_CACHE_DIR", "/var/cache/lumelo/artwork")
	provisioningStatusPath := getenv("CONTROLD_PROVISIONING_STATUS_PATH", filepath.Join(runtimeDir, "provisioning-status.json"))
	alsaCardsPath := getenv("CONTROLD_ALSA_CARDS_PATH", "")
	mediaImportCommand := getenv("CONTROLD_MEDIA_IMPORT_BIN", "lumelo-media-import")

	server, err := api.New(api.Dependencies{
		Auth:             auth.NewService(false),
		Playback:         playbackclient.New(commandSocket, eventSocket),
		Library:          libraryclient.New(libraryDBPath),
		MediaImport:      mediaimport.New(mediaImportCommand, libraryDBPath),
		Logs:             logclient.New(),
		Provisioning:     provisioningclient.New(provisioningStatusPath),
		AudioOutput:      audiodevice.New(alsaCardsPath),
		Settings:         cfg,
		SSH:              sshctl.NewController(cfg.SSHEnabled),
		Templates:        web.Assets,
		Static:           web.Assets,
		ArtworkCacheRoot: artworkCacheRoot,
	})
	if err != nil {
		log.Fatalf("build controld server: %v", err)
	}

	addr := getenv("CONTROLD_LISTEN_ADDR", ":8080")
	log.Printf("lumelo controld listening on %s", addr)

	if err := http.ListenAndServe(addr, server.Handler()); err != nil {
		log.Fatalf("serve controld: %v", err)
	}
}

func getenv(key, fallback string) string {
	value := os.Getenv(key)
	if value == "" {
		return fallback
	}

	return value
}

func getenvWithFallbacks(keys []string, fallback string) string {
	for _, key := range keys {
		if value := os.Getenv(key); value != "" {
			return value
		}
	}

	return fallback
}
