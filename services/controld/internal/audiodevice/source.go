package audiodevice

import (
	"context"
	"fmt"
	"os"
	"regexp"
	"strconv"
	"strings"
)

const defaultCardsPath = "/proc/asound/cards"

var cardLinePattern = regexp.MustCompile(`^\s*([0-9]+)\s+\[([^\]]+)\]\s*:\s*(.+?)\s+-\s+(.+)$`)

type Source struct {
	CardsPath string
}

type Snapshot struct {
	Connected bool   `json:"connected"`
	Current   Device `json:"current"`
	ReadError string `json:"read_error,omitempty"`
}

type Device struct {
	CardIndex  int    `json:"card_index"`
	CardID     string `json:"card_id"`
	Name       string `json:"name"`
	Driver     string `json:"driver,omitempty"`
	ALSADevice string `json:"alsa_device"`
}

func New(cardsPath string) *Source {
	if strings.TrimSpace(cardsPath) == "" {
		cardsPath = defaultCardsPath
	}
	return &Source{CardsPath: cardsPath}
}

func (s *Source) Snapshot(_ context.Context) Snapshot {
	cardsPath := s.CardsPath
	if strings.TrimSpace(cardsPath) == "" {
		cardsPath = defaultCardsPath
	}

	contents, err := os.ReadFile(cardsPath)
	if err != nil {
		return Snapshot{ReadError: fmt.Sprintf("read %s: %v", cardsPath, err)}
	}
	return SnapshotFromCards(string(contents))
}

func SnapshotFromCards(contents string) Snapshot {
	devices := USBDecoderDevicesFromCards(contents)
	if len(devices) == 0 {
		return Snapshot{}
	}
	return Snapshot{
		Connected: true,
		Current:   devices[0],
	}
}

func USBDecoderDevicesFromCards(contents string) []Device {
	lines := strings.Split(contents, "\n")
	devices := make([]Device, 0)

	for i := 0; i < len(lines); i++ {
		matches := cardLinePattern.FindStringSubmatch(lines[i])
		if matches == nil {
			continue
		}

		cardIndex, err := strconv.Atoi(matches[1])
		if err != nil {
			continue
		}
		cardID := strings.TrimSpace(matches[2])
		driver := strings.TrimSpace(matches[3])
		name := strings.TrimSpace(matches[4])
		detail := ""
		if i+1 < len(lines) && cardLinePattern.FindStringSubmatch(lines[i+1]) == nil {
			detail = strings.TrimSpace(lines[i+1])
		}

		if !isUSBAudioCard(driver, detail) {
			continue
		}

		devices = append(devices, Device{
			CardIndex:  cardIndex,
			CardID:     cardID,
			Name:       decoderDisplayName(name, detail),
			Driver:     driver,
			ALSADevice: "plughw:CARD=" + cardID + ",DEV=0",
		})
	}

	return devices
}

func isUSBAudioCard(driver, detail string) bool {
	driver = strings.ToLower(strings.TrimSpace(driver))
	detail = strings.ToLower(strings.TrimSpace(detail))
	return driver == "usb-audio" || strings.Contains(detail, " at usb-")
}

func decoderDisplayName(name, detail string) string {
	name = strings.TrimSpace(name)
	detail = strings.TrimSpace(detail)

	if detail != "" {
		for _, marker := range []string{" at usb-", " at "} {
			if index := strings.Index(detail, marker); index > 0 {
				detail = strings.TrimSpace(detail[:index])
				break
			}
		}
		if detail != "" {
			return detail
		}
	}

	return name
}
