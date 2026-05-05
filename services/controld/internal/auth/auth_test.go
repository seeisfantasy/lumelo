package auth

import (
	"os"
	"path/filepath"
	"testing"
	"time"
)

func TestFileServiceSetsAndReloadsPassword(t *testing.T) {
	path := filepath.Join(t.TempDir(), "auth.json")
	service, err := NewFileService(path)
	if err != nil {
		t.Fatalf("NewFileService: %v", err)
	}
	if service.PasswordConfigured() {
		t.Fatalf("new auth state should not have a configured password")
	}

	if err := service.SetPassword("password123"); err != nil {
		t.Fatalf("SetPassword: %v", err)
	}
	if !service.VerifyPassword("password123") {
		t.Fatalf("expected password to verify")
	}
	if service.VerifyPassword("wrong-password") {
		t.Fatalf("wrong password should not verify")
	}

	reloaded, err := NewFileService(path)
	if err != nil {
		t.Fatalf("reload auth state: %v", err)
	}
	if !reloaded.PasswordConfigured() || !reloaded.VerifyPassword("password123") {
		t.Fatalf("reloaded auth state did not preserve password")
	}

	info, err := os.Stat(path)
	if err != nil {
		t.Fatalf("stat auth state: %v", err)
	}
	if mode := info.Mode().Perm(); mode != 0o600 {
		t.Fatalf("auth state mode = %o, want 0600", mode)
	}
}

func TestSessionLifecycle(t *testing.T) {
	service := NewMemoryService(false)
	token, err := service.CreateSession()
	if err != nil {
		t.Fatalf("CreateSession: %v", err)
	}
	if !service.ValidateSession(token) {
		t.Fatalf("expected session to validate")
	}
	service.DeleteSession(token)
	if service.ValidateSession(token) {
		t.Fatalf("deleted session should not validate")
	}
}

func TestAuthenticateLocksAfterRepeatedFailuresAndResetsOnSuccess(t *testing.T) {
	service := NewMemoryService(false)
	now := time.Date(2026, 5, 5, 12, 0, 0, 0, time.UTC)
	service.now = func() time.Time { return now }
	if err := service.SetPassword("password123"); err != nil {
		t.Fatalf("SetPassword: %v", err)
	}

	for attempt := 0; attempt < maxLoginFailures; attempt++ {
		ok, retryAfter := service.Authenticate("wrong-password")
		if ok {
			t.Fatalf("wrong password authenticated")
		}
		if attempt < maxLoginFailures-1 && retryAfter != 0 {
			t.Fatalf("unexpected early lockout after attempt %d: %s", attempt+1, retryAfter)
		}
	}

	ok, retryAfter := service.Authenticate("password123")
	if ok || retryAfter == 0 {
		t.Fatalf("expected lockout before retry window expires, ok=%v retryAfter=%s", ok, retryAfter)
	}

	now = now.Add(loginLockoutTTL + time.Second)
	ok, retryAfter = service.Authenticate("password123")
	if !ok || retryAfter != 0 {
		t.Fatalf("expected login success after lockout expires, ok=%v retryAfter=%s", ok, retryAfter)
	}

	ok, retryAfter = service.Authenticate("wrong-password")
	if ok || retryAfter != 0 {
		t.Fatalf("expected failure counter to reset after success, ok=%v retryAfter=%s", ok, retryAfter)
	}
}
