package web

import "embed"

// Assets embeds the minimal SSR templates and static files used by the
// Lumelo controld prototype.
//
//go:embed templates/*.html static/css/*.css
var Assets embed.FS
