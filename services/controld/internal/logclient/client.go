package logclient

import (
	"context"
	"fmt"
	"os/exec"
	"strconv"
	"strings"
	"time"
)

const (
	defaultLines = 300
	maxLines     = 1000
	readTimeout  = 4 * time.Second
)

type Client struct{}

func New() *Client {
	return &Client{}
}

func (c *Client) Recent(ctx context.Context, lines int) (string, error) {
	lines = clampLines(lines)
	ctx, cancel := context.WithTimeout(ctx, readTimeout)
	defer cancel()

	cmd := exec.CommandContext(ctx, "journalctl", "-b", "--no-pager", "-n", strconv.Itoa(lines))
	output, err := cmd.CombinedOutput()
	text := string(output)
	if err != nil {
		if strings.TrimSpace(text) == "" {
			text = err.Error() + "\n"
		}
		return text, fmt.Errorf("journalctl: %w", err)
	}
	if strings.TrimSpace(text) == "" {
		return "(journalctl returned no lines)\n", nil
	}

	return text, nil
}

func clampLines(lines int) int {
	if lines <= 0 {
		return defaultLines
	}
	if lines > maxLines {
		return maxLines
	}

	return lines
}
