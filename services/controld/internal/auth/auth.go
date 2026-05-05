package auth

import (
	"crypto/hmac"
	"crypto/rand"
	"crypto/sha256"
	"crypto/subtle"
	"encoding/base64"
	"encoding/binary"
	"encoding/json"
	"errors"
	"fmt"
	"os"
	"path/filepath"
	"sync"
	"time"
)

const (
	DefaultCookieName = "lumelo_session"
	minPasswordLength = 8
	defaultIterations = 200_000
	sessionTTL        = 12 * time.Hour
	maxLoginFailures  = 5
	loginLockoutTTL   = time.Minute
)

type fileState struct {
	Version      int    `json:"version"`
	Algorithm    string `json:"algorithm"`
	Iterations   int    `json:"iterations"`
	Salt         string `json:"salt"`
	PasswordHash string `json:"password_hash"`
	UpdatedAt    string `json:"updated_at"`
}

type session struct {
	ExpiresAt time.Time
}

// Service owns the single-admin authentication state for V1.
type Service struct {
	mu                 sync.Mutex
	required           bool
	statePath          string
	passwordConfigured bool
	iterations         int
	salt               []byte
	passwordHash       []byte
	sessions           map[string]session
	failedLoginCount   int
	loginLockedUntil   time.Time
	now                func() time.Time
}

// NewService returns a disabled in-memory service used by tests and dev-only
// callers that explicitly do not want route protection.
func NewService(passwordConfigured bool) *Service {
	return &Service{
		required:           false,
		passwordConfigured: passwordConfigured,
		iterations:         defaultIterations,
		sessions:           map[string]session{},
		now:                time.Now,
	}
}

func NewMemoryService(passwordConfigured bool) *Service {
	service := NewService(passwordConfigured)
	service.required = true
	return service
}

func NewFileService(path string) (*Service, error) {
	service := NewMemoryService(false)
	service.statePath = path
	if err := service.load(); err != nil {
		return nil, err
	}
	return service, nil
}

func (s *Service) Required() bool {
	if s == nil {
		return false
	}
	return s.required
}

func (s *Service) PasswordConfigured() bool {
	if s == nil {
		return false
	}
	s.mu.Lock()
	defer s.mu.Unlock()
	return s.passwordConfigured
}

func (s *Service) SetPassword(password string) error {
	if len(password) < minPasswordLength {
		return fmt.Errorf("password must be at least %d characters", minPasswordLength)
	}

	salt := make([]byte, 16)
	if _, err := rand.Read(salt); err != nil {
		return fmt.Errorf("generate password salt: %w", err)
	}
	hash := pbkdf2SHA256([]byte(password), salt, defaultIterations, sha256.Size)

	s.mu.Lock()
	defer s.mu.Unlock()
	s.iterations = defaultIterations
	s.salt = salt
	s.passwordHash = hash
	s.passwordConfigured = true
	return s.persistLocked()
}

func (s *Service) VerifyPassword(password string) bool {
	if s == nil {
		return false
	}

	s.mu.Lock()
	defer s.mu.Unlock()
	return s.verifyPasswordLocked(password)
}

func (s *Service) Authenticate(password string) (bool, time.Duration) {
	if s == nil {
		return false, 0
	}

	s.mu.Lock()
	defer s.mu.Unlock()
	current := s.now()
	if s.loginLockedUntil.After(current) {
		return false, s.loginLockedUntil.Sub(current)
	}
	if s.verifyPasswordLocked(password) {
		s.failedLoginCount = 0
		s.loginLockedUntil = time.Time{}
		return true, 0
	}

	s.failedLoginCount++
	if s.failedLoginCount >= maxLoginFailures {
		s.loginLockedUntil = current.Add(loginLockoutTTL)
		return false, loginLockoutTTL
	}
	return false, 0
}

func (s *Service) verifyPasswordLocked(password string) bool {
	if !s.passwordConfigured || len(s.salt) == 0 || len(s.passwordHash) == 0 {
		return false
	}
	hash := pbkdf2SHA256([]byte(password), s.salt, s.iterations, len(s.passwordHash))
	return subtle.ConstantTimeCompare(hash, s.passwordHash) == 1
}

func (s *Service) CreateSession() (string, error) {
	tokenBytes := make([]byte, 32)
	if _, err := rand.Read(tokenBytes); err != nil {
		return "", fmt.Errorf("generate session token: %w", err)
	}
	token := base64.RawURLEncoding.EncodeToString(tokenBytes)

	s.mu.Lock()
	defer s.mu.Unlock()
	s.sessions[token] = session{ExpiresAt: s.now().Add(sessionTTL)}
	return token, nil
}

func (s *Service) ValidateSession(token string) bool {
	if s == nil || token == "" {
		return false
	}

	s.mu.Lock()
	defer s.mu.Unlock()
	current := s.now()
	stored, ok := s.sessions[token]
	if !ok {
		return false
	}
	if !stored.ExpiresAt.After(current) {
		delete(s.sessions, token)
		return false
	}
	return true
}

func (s *Service) DeleteSession(token string) {
	if s == nil || token == "" {
		return
	}
	s.mu.Lock()
	defer s.mu.Unlock()
	delete(s.sessions, token)
}

func (s *Service) load() error {
	if s.statePath == "" {
		return nil
	}

	data, err := os.ReadFile(s.statePath)
	if err != nil {
		if errors.Is(err, os.ErrNotExist) {
			return nil
		}
		return fmt.Errorf("read auth state %s: %w", s.statePath, err)
	}

	var state fileState
	if err := json.Unmarshal(data, &state); err != nil {
		return fmt.Errorf("parse auth state %s: %w", s.statePath, err)
	}
	if state.Version != 1 || state.Algorithm != "pbkdf2-hmac-sha256" {
		return fmt.Errorf("unsupported auth state schema: version=%d algorithm=%q", state.Version, state.Algorithm)
	}
	if state.Iterations <= 0 {
		return fmt.Errorf("invalid auth state iterations: %d", state.Iterations)
	}
	salt, err := base64.StdEncoding.DecodeString(state.Salt)
	if err != nil {
		return fmt.Errorf("decode auth salt: %w", err)
	}
	hash, err := base64.StdEncoding.DecodeString(state.PasswordHash)
	if err != nil {
		return fmt.Errorf("decode auth password hash: %w", err)
	}
	if len(salt) == 0 || len(hash) == 0 {
		return fmt.Errorf("auth state is missing password material")
	}

	s.salt = salt
	s.passwordHash = hash
	s.iterations = state.Iterations
	s.passwordConfigured = true
	return nil
}

func (s *Service) persistLocked() error {
	if s.statePath == "" {
		return nil
	}

	state := fileState{
		Version:      1,
		Algorithm:    "pbkdf2-hmac-sha256",
		Iterations:   s.iterations,
		Salt:         base64.StdEncoding.EncodeToString(s.salt),
		PasswordHash: base64.StdEncoding.EncodeToString(s.passwordHash),
		UpdatedAt:    s.now().UTC().Format(time.RFC3339Nano),
	}
	data, err := json.MarshalIndent(state, "", "  ")
	if err != nil {
		return fmt.Errorf("marshal auth state: %w", err)
	}
	data = append(data, '\n')

	if err := os.MkdirAll(filepath.Dir(s.statePath), 0o750); err != nil {
		return fmt.Errorf("create auth state dir: %w", err)
	}
	tmp, err := os.CreateTemp(filepath.Dir(s.statePath), ".auth-*.tmp")
	if err != nil {
		return fmt.Errorf("create auth temp file: %w", err)
	}
	tmpName := tmp.Name()
	defer func() { _ = os.Remove(tmpName) }()

	if _, err := tmp.Write(data); err != nil {
		_ = tmp.Close()
		return fmt.Errorf("write auth temp file: %w", err)
	}
	if err := tmp.Chmod(0o600); err != nil {
		_ = tmp.Close()
		return fmt.Errorf("chmod auth temp file: %w", err)
	}
	if err := tmp.Sync(); err != nil {
		_ = tmp.Close()
		return fmt.Errorf("sync auth temp file: %w", err)
	}
	if err := tmp.Close(); err != nil {
		return fmt.Errorf("close auth temp file: %w", err)
	}
	if err := os.Rename(tmpName, s.statePath); err != nil {
		return fmt.Errorf("replace auth state: %w", err)
	}
	dir, err := os.Open(filepath.Dir(s.statePath))
	if err != nil {
		return fmt.Errorf("open auth state dir for sync: %w", err)
	}
	defer dir.Close()
	if err := dir.Sync(); err != nil {
		return fmt.Errorf("sync auth state dir: %w", err)
	}
	return nil
}

func pbkdf2SHA256(password, salt []byte, iterations, keyLen int) []byte {
	hashLen := sha256.Size
	numBlocks := (keyLen + hashLen - 1) / hashLen
	output := make([]byte, 0, numBlocks*hashLen)
	for block := 1; block <= numBlocks; block++ {
		output = append(output, pbkdf2Block(password, salt, iterations, uint32(block))...)
	}
	return output[:keyLen]
}

func pbkdf2Block(password, salt []byte, iterations int, block uint32) []byte {
	mac := hmac.New(sha256.New, password)
	_, _ = mac.Write(salt)
	var blockBytes [4]byte
	binary.BigEndian.PutUint32(blockBytes[:], block)
	_, _ = mac.Write(blockBytes[:])
	u := mac.Sum(nil)
	result := make([]byte, len(u))
	copy(result, u)

	for i := 1; i < iterations; i++ {
		mac = hmac.New(sha256.New, password)
		_, _ = mac.Write(u)
		u = mac.Sum(nil)
		for j := range result {
			result[j] ^= u[j]
		}
	}
	return result
}
