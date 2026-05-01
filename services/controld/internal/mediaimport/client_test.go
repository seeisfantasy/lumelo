package mediaimport

import "testing"

func TestValidateDevicePathRequiresDevPath(t *testing.T) {
	if _, err := validateDevicePath("/media/music"); err == nil {
		t.Fatalf("expected non-/dev path to be rejected")
	}
	path, err := validateDevicePath("/dev/sda1")
	if err != nil {
		t.Fatalf("expected /dev/sda1 to pass: %v", err)
	}
	if path != "/dev/sda1" {
		t.Fatalf("unexpected normalized path: %q", path)
	}
}

func TestValidateScanPathRequiresMediaOrMnt(t *testing.T) {
	if _, err := validateScanPath("/etc"); err == nil {
		t.Fatalf("expected non-media path to be rejected")
	}
	path, err := validateScanPath("/media/music")
	if err != nil {
		t.Fatalf("expected media path to pass: %v", err)
	}
	if path != "/media/music" {
		t.Fatalf("unexpected normalized path: %q", path)
	}
}
