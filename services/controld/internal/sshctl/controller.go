package sshctl

import (
	"fmt"
	"os/exec"
	"strings"
)

type Controller struct {
	enabled      bool
	applyCommand string
}

func NewController(enabled bool) *Controller {
	return &Controller{enabled: enabled}
}

func NewControllerWithCommand(enabled bool, applyCommand string) *Controller {
	return &Controller{
		enabled:      enabled,
		applyCommand: strings.TrimSpace(applyCommand),
	}
}

func (c *Controller) Enabled() bool {
	return c.enabled
}

func (c *Controller) SetEnabled(enabled bool) error {
	if c.applyCommand != "" {
		action := "disable"
		if enabled {
			action = "enable"
		}
		output, err := exec.Command(c.applyCommand, action).CombinedOutput()
		if err != nil {
			return fmt.Errorf("apply ssh %s: %w: %s", action, err, strings.TrimSpace(string(output)))
		}
	}
	c.enabled = enabled
	return nil
}
