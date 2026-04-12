package auth

// Service owns the single-admin authentication state for V1.
type Service struct {
	passwordConfigured bool
}

func NewService(passwordConfigured bool) *Service {
	return &Service{passwordConfigured: passwordConfigured}
}

func (s *Service) PasswordConfigured() bool {
	return s.passwordConfigured
}
