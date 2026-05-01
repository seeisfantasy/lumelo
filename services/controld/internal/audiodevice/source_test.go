package audiodevice

import "testing"

func TestSnapshotFromCardsReportsUnconnectedWhenNoUSBAudio(t *testing.T) {
	snapshot := SnapshotFromCards(` 0 [HDMI           ]: HDA-Intel - HDMI Audio
                      HDMI Audio at 0xfe800000 irq 50
`)

	if snapshot.Connected {
		t.Fatalf("expected no USB decoder, got %+v", snapshot)
	}
	if snapshot.Current.Name != "" {
		t.Fatalf("expected empty current device, got %+v", snapshot.Current)
	}
}

func TestSnapshotFromCardsSelectsUSBDecoder(t *testing.T) {
	snapshot := SnapshotFromCards(` 0 [HDMI           ]: HDA-Intel - HDMI Audio
                      HDMI Audio at 0xfe800000 irq 50
 1 [Audio          ]: USB-Audio - USB Audio
                      XMOS xCORE USB Audio 2.0 at usb-xhci-hcd.1.auto-1, high speed
`)

	if !snapshot.Connected {
		t.Fatalf("expected USB decoder to be connected")
	}
	if snapshot.Current.CardIndex != 1 || snapshot.Current.CardID != "Audio" {
		t.Fatalf("unexpected selected card: %+v", snapshot.Current)
	}
	if snapshot.Current.Name != "XMOS xCORE USB Audio 2.0" {
		t.Fatalf("unexpected display name: %q", snapshot.Current.Name)
	}
	if snapshot.Current.Driver != "USB-Audio" {
		t.Fatalf("unexpected driver: %q", snapshot.Current.Driver)
	}
	if snapshot.Current.ALSADevice != "plughw:CARD=Audio,DEV=0" {
		t.Fatalf("unexpected ALSA device: %q", snapshot.Current.ALSADevice)
	}
}

func TestUSBDecoderDevicesFromCardsKeepsMultipleDevicesForFutureSelection(t *testing.T) {
	devices := USBDecoderDevicesFromCards(` 1 [Audio          ]: USB-Audio - USB Audio
                      DAC One at usb-xhci-hcd.1.auto-1, high speed
 2 [Device         ]: USB-Audio - USB Audio Device
                      DAC Two at usb-xhci-hcd.1.auto-2, full speed
`)

	if len(devices) != 2 {
		t.Fatalf("unexpected devices: %+v", devices)
	}
	if devices[0].Name != "DAC One" || devices[1].Name != "DAC Two" {
		t.Fatalf("unexpected names: %+v", devices)
	}
}
