package sshctl

type Controller struct {
	enabled bool
}

func NewController(enabled bool) *Controller {
	return &Controller{enabled: enabled}
}

func (c *Controller) Enabled() bool {
	return c.enabled
}
