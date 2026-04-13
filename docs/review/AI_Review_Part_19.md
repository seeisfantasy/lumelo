# AI Review Part 19

这是给外部 AI 做静态审查的代码分卷。每一卷都只包含仓库快照中的一部分文本文件内容，按当前工作树生成。

## `services/controld/web/templates/index.html`

- bytes: 12327
- segment: 1/1

~~~html
<!doctype html>
<html lang="en">
  <head>
    <meta charset="utf-8">
    <meta name="viewport" content="width=device-width, initial-scale=1">
    <title>Lumelo controld</title>
    <link rel="stylesheet" href="/static/css/app.css">
  </head>
  <body>
    <main class="shell">
      <nav class="topnav">
        <a href="/" class="topnav-link {{if eq .CurrentPage "home"}}topnav-link-current{{end}}">Home</a>
        <a href="/library" class="topnav-link {{if eq .CurrentPage "library"}}topnav-link-current{{end}}">Library</a>
        <a href="/provisioning" class="topnav-link {{if eq .CurrentPage "provisioning"}}topnav-link-current{{end}}">Provisioning</a>
        <a href="/logs" class="topnav-link {{if eq .CurrentPage "logs"}}topnav-link-current{{end}}">Logs</a>
      </nav>

      <section class="card">
        <p class="eyebrow">Lumelo V1</p>
        <h1>Lumelo controld is online</h1>
        <p class="summary">
          This page exists to prove the initial Go module, embedded SSR assets,
          and service boundary layout are wired together.
        </p>
        <p class="summary" id="playback-live-status">
          Live sync listens only for necessary playback events. It does not use
          high-frequency polling.
        </p>
      </section>

      <section class="card">
        <h2>Current defaults</h2>
        <dl class="grid">
          <div>
            <dt>Mode</dt>
            <dd>{{.Mode}}</dd>
          </div>
          <div>
            <dt>Interface</dt>
            <dd>{{.InterfaceMode}}</dd>
          </div>
          <div>
            <dt>DSD policy</dt>
            <dd>{{.DSDPolicy}}</dd>
          </div>
          <div>
            <dt>Admin password set</dt>
            <dd>{{.PasswordConfigured}}</dd>
          </div>
          <div>
            <dt>SSH enabled</dt>
            <dd>{{.SSHEnabled}}</dd>
          </div>
          <div>
            <dt>Config path</dt>
            <dd>{{.ConfigPath}}</dd>
          </div>
          <div>
            <dt>Playback cmd socket</dt>
            <dd>{{.CommandSocket}}</dd>
          </div>
          <div>
            <dt>Playback event socket</dt>
            <dd>{{.EventSocket}}</dd>
          </div>
          <div>
            <dt>Library database</dt>
            <dd>{{.LibraryDBPath}}</dd>
          </div>
        </dl>
      </section>

      <section class="card">
        <h2>Provisioning Summary</h2>
        {{if .Provisioning.Available}}
          <p class="status-line">
            State:
            {{if eq .Provisioning.State "connected"}}
              <span class="pill pill-ok">connected</span>
            {{else if eq .Provisioning.State "failed"}}
              <span class="pill pill-offline">failed</span>
            {{else}}
              <span class="pill pill-current">{{if .Provisioning.State}}{{.Provisioning.State}}{{else}}unknown{{end}}</span>
            {{end}}
          </p>
        {{else}}
          <p class="status-line">
            State:
            <span class="pill pill-offline">unavailable</span>
          </p>
        {{end}}

        <dl class="grid">
          <div>
            <dt>Message</dt>
            <dd>{{if .Provisioning.Message}}{{.Provisioning.Message}}{{else}}-{{end}}</dd>
          </div>
          <div>
            <dt>SSID</dt>
            <dd>{{if .Provisioning.SSID}}{{.Provisioning.SSID}}{{else}}-{{end}}</dd>
          </div>
          <div>
            <dt>IP</dt>
            <dd>{{if .Provisioning.IP}}{{.Provisioning.IP}}{{else}}-{{end}}</dd>
          </div>
          <div>
            <dt>Wi-Fi IP</dt>
            <dd>{{if .Provisioning.WiFiIP}}{{.Provisioning.WiFiIP}}{{else}}-{{end}}</dd>
          </div>
          <div>
            <dt>Wired IP</dt>
            <dd>{{if .Provisioning.WiredIP}}{{.Provisioning.WiredIP}}{{else}}-{{end}}</dd>
          </div>
          <div>
            <dt>All IPs</dt>
            <dd>{{if .Provisioning.AllIPs}}{{range $index, $ip := .Provisioning.AllIPs}}{{if $index}}, {{end}}{{$ip}}{{end}}{{else}}-{{end}}</dd>
          </div>
          <div>
            <dt>Wi-Fi interface</dt>
            <dd>{{if .Provisioning.WiFiInterface}}{{.Provisioning.WiFiInterface}}{{else}}-{{end}}</dd>
          </div>
          <div>
            <dt>Error code</dt>
            <dd>{{if .Provisioning.ErrorCode}}{{.Provisioning.ErrorCode}}{{else}}-{{end}}</dd>
          </div>
          <div>
            <dt>Updated at</dt>
            <dd>{{if .Provisioning.UpdatedAt}}{{.Provisioning.UpdatedAt}}{{else}}-{{end}}</dd>
          </div>
        </dl>

        {{if .Provisioning.ApplyOutput}}
          <p class="summary">Apply output: {{.Provisioning.ApplyOutput}}</p>
        {{end}}
        {{if .Provisioning.DiagnosticHint}}
          <p class="summary">Diagnostic hint: {{.Provisioning.DiagnosticHint}}</p>
        {{end}}
        {{if .Provisioning.ReadError}}
          <p class="banner banner-error">Provisioning status read error: {{.Provisioning.ReadError}}</p>
        {{end}}

        <div class="actions log-actions">
          <a href="/provisioning" class="button-link">Open Provisioning</a>
          <a href="/provisioning-status" class="button-link button-link-secondary">Open JSON</a>
          <a href="/healthz" class="button-link button-link-secondary">Healthz</a>
          <a href="/logs?lines=300" class="button-link button-link-secondary">Logs</a>
        </div>
      </section>

      <section class="card">
        <h2>Playbackd Status</h2>
        {{if .CommandMessage}}
          <p class="banner banner-ok">{{.CommandMessage}}</p>
        {{end}}
        {{if .CommandError}}
          <p class="banner banner-error">{{.CommandError}}</p>
        {{end}}

        <p class="status-line">
          Socket:
          {{if .PlaybackStatus.Available}}
            <span class="pill pill-ok">online</span>
          {{else}}
            <span class="pill pill-offline">offline</span>
          {{end}}
        </p>
        {{if .PlaybackStatus.Error}}
          <p class="summary">Last IPC error: {{.PlaybackStatus.Error}}</p>
        {{end}}

        <dl class="grid">
          <div>
            <dt>State</dt>
            <dd>{{if .PlaybackStatus.State}}{{.PlaybackStatus.State}}{{else}}unknown{{end}}</dd>
          </div>
          <div>
            <dt>Order mode</dt>
            <dd>{{if .PlaybackStatus.OrderMode}}{{.PlaybackStatus.OrderMode}}{{else}}-{{end}}</dd>
          </div>
          <div>
            <dt>Repeat mode</dt>
            <dd>{{if .PlaybackStatus.RepeatMode}}{{.PlaybackStatus.RepeatMode}}{{else}}-{{end}}</dd>
          </div>
          <div>
            <dt>Current track</dt>
            <dd>{{if .PlaybackStatus.CurrentTrack}}{{.PlaybackStatus.CurrentTrack}}{{else}}-{{end}}</dd>
          </div>
          <div>
            <dt>Last command</dt>
            <dd>{{if .PlaybackStatus.LastCommand}}{{.PlaybackStatus.LastCommand}}{{else}}-{{end}}</dd>
          </div>
          <div>
            <dt>Queue entries</dt>
            <dd>{{.PlaybackStatus.QueueEntries}}</dd>
          </div>
        </dl>
      </section>

      <section class="card">
        <h2>Playback Controls</h2>
        <form method="post" action="/commands" class="stack">
          <label class="label" for="track-id">Track id</label>
          <input id="track-id" name="track_id" value="{{.SuggestedTrackID}}" class="input">
          <div class="actions">
            <button type="submit" name="action" value="play">Play</button>
            <button type="submit" name="action" value="play_history">Play History</button>
          </div>
        </form>

        <form method="post" action="/commands" class="actions">
          <button type="submit" name="action" value="pause">Pause</button>
          <button type="submit" name="action" value="prev">Prev</button>
          <button type="submit" name="action" value="next">Next</button>
          <button type="submit" name="action" value="stop">Stop</button>
          <button type="submit" name="action" value="status">Refresh Status</button>
          <button type="submit" name="action" value="ping">Ping Socket</button>
        </form>
      </section>

      <section class="card">
        <h2>Queue</h2>
        {{if .QueueSnapshot.Error}}
          <p class="summary">Last queue snapshot error: {{.QueueSnapshot.Error}}</p>
        {{end}}

        <dl class="grid">
          <div>
            <dt>Snapshot socket</dt>
            <dd>
              {{if .QueueSnapshot.Available}}
                <span class="pill pill-ok">online</span>
              {{else}}
                <span class="pill pill-offline">offline</span>
              {{end}}
            </dd>
          </div>
          <div>
            <dt>Current order index</dt>
            <dd>{{.CurrentOrderLabel}}</dd>
          </div>
        </dl>

        <form method="post" action="/commands" class="stack queue-tools">
          <label class="label" for="queue-track-id">Queue track id</label>
          <input id="queue-track-id" name="track_id" value="{{.SuggestedTrackID}}" class="input">
          <div class="actions">
            <button type="submit" name="action" value="queue_append">Append</button>
            <button type="submit" name="action" value="queue_insert_next">Insert Next</button>
            <button type="submit" name="action" value="queue_snapshot">Refresh Queue</button>
            <button type="submit" name="action" value="queue_clear">Clear Queue</button>
          </div>
        </form>

        {{if .QueueEntries}}
          <div class="queue-list">
            {{range .QueueEntries}}
              <article class="queue-item {{if .IsCurrent}}queue-item-current{{end}}">
                <div class="queue-order">{{.DisplayIndex}}</div>
                <div class="queue-copy">
                  <p class="queue-title">
                    {{.Title}}
                    {{if .IsCurrent}}<span class="pill pill-current">current</span>{{end}}
                  </p>
                  <p class="queue-meta">entry {{.QueueEntryID}} · track {{.TrackUID}}</p>
                  <p class="queue-meta">{{.RelativePath}}</p>
                </div>
                <form method="post" action="/commands" class="queue-row-action">
                  <input type="hidden" name="track_id" value="{{.QueueEntryID}}">
                  <button type="submit" name="action" value="queue_remove">Remove</button>
                </form>
              </article>
            {{end}}
          </div>
        {{else if .QueueSnapshot.Available}}
          <p class="summary">Queue is empty.</p>
        {{else}}
          <p class="summary">Queue snapshot is unavailable.</p>
        {{end}}
      </section>
    </main>
    <script>
      (() => {
        const statusNode = document.getElementById("playback-live-status");
        if (!statusNode || !window.EventSource) {
          return;
        }

        const stream = new EventSource('{{.PlaybackStreamPath}}');
        let reloadPending = false;

        const scheduleReload = (label) => {
          if (reloadPending) {
            return;
          }
          reloadPending = true;
          statusNode.textContent = `Playback event ${label} detected. Refreshing this SSR view.`;
          window.setTimeout(() => window.location.reload(), 140);
        };

        const watchedEvents = [
          "PLAYBACK_STARTED",
          "PLAYBACK_PAUSED",
          "PLAYBACK_RESUMED",
          "PLAYBACK_STOPPED",
          "TRACK_CHANGED",
          "PLAYBACK_FAILED"
        ];

        watchedEvents.forEach((eventName) => {
          stream.addEventListener(eventName, () => scheduleReload(eventName.toLowerCase()));
        });

        stream.addEventListener("STREAM_ERROR", () => {
          reloadPending = false;
          statusNode.textContent = "Live playback sync disconnected. The page will reconnect automatically.";
        });

        stream.onopen = () => {
          reloadPending = false;
          statusNode.textContent = "Live playback sync is on. The page refreshes only when playback events happen.";
        };

        stream.onerror = () => {
          reloadPending = false;
          statusNode.textContent = "Live playback sync is reconnecting. Manual controls still work.";
        };
      })();
    </script>
  </body>
</html>
~~~

## `services/controld/web/templates/library.html`

- bytes: 6620
- segment: 1/1

~~~html
<!doctype html>
<html lang="en">
  <head>
    <meta charset="utf-8">
    <meta name="viewport" content="width=device-width, initial-scale=1">
    <title>Lumelo library</title>
    <link rel="stylesheet" href="/static/css/app.css">
  </head>
  <body>
    <main class="shell">
      <nav class="topnav">
        <a href="/" class="topnav-link {{if eq .CurrentPage "home"}}topnav-link-current{{end}}">Home</a>
        <a href="/library" class="topnav-link {{if eq .CurrentPage "library"}}topnav-link-current{{end}}">Library</a>
        <a href="/provisioning" class="topnav-link {{if eq .CurrentPage "provisioning"}}topnav-link-current{{end}}">Provisioning</a>
        <a href="/logs" class="topnav-link {{if eq .CurrentPage "logs"}}topnav-link-current{{end}}">Logs</a>
      </nav>

      <section class="card">
        <p class="eyebrow">Lumelo V1</p>
        <h1>Library overview</h1>
        <p class="summary">
          This page reads the current SQLite library state directly from
          <code>library.db</code> and is intended to validate the current index
          layer and media boundaries.
        </p>
      </section>

      <section class="card">
        <h2>Playback Boundary</h2>
        <p class="summary">
          New TF or USB media can be mounted while you listen, but full scan
          remains blocked during playback. Stop playback first, then trigger the
          next manual scan.
        </p>
        <dl class="grid">
          <div>
            <dt>Playback state</dt>
            <dd>{{if .PlaybackStatus.State}}{{.PlaybackStatus.State}}{{else}}unknown{{end}}</dd>
          </div>
          <div>
            <dt>Current track</dt>
            <dd>{{if .PlaybackStatus.CurrentTrack}}{{.PlaybackStatus.CurrentTrack}}{{else}}-{{end}}</dd>
          </div>
        </dl>
        {{if .PlaybackScanBlock}}
          <p class="banner banner-warn">
            Playback Quiet Mode is active. If you just inserted new media, do
            not expect new albums to appear yet. Stop playback, then run the
            next manual scan.
          </p>
        {{else}}
          <p class="summary">
            Playback is not currently blocking scan. New media still requires a
            manual scan before album results become complete.
          </p>
        {{end}}
      </section>

      <section class="card">
        <h2>Library Status</h2>
        <p class="status-line">
          Database:
          {{if .LibrarySnapshot.Available}}
            <span class="pill pill-ok">online</span>
          {{else}}
            <span class="pill pill-offline">offline</span>
          {{end}}
        </p>
        <dl class="grid">
          <div>
            <dt>Library database</dt>
            <dd>{{.LibraryDBPath}}</dd>
          </div>
          <div>
            <dt>Volumes</dt>
            <dd>{{.LibrarySnapshot.Stats.VolumeCount}}</dd>
          </div>
          <div>
            <dt>Albums</dt>
            <dd>{{.LibrarySnapshot.Stats.AlbumCount}}</dd>
          </div>
          <div>
            <dt>Tracks</dt>
            <dd>{{.LibrarySnapshot.Stats.TrackCount}}</dd>
          </div>
          <div>
            <dt>Artists</dt>
            <dd>{{.LibrarySnapshot.Stats.ArtistCount}}</dd>
          </div>
          <div>
            <dt>Genres</dt>
            <dd>{{.LibrarySnapshot.Stats.GenreCount}}</dd>
          </div>
        </dl>
        {{if .LibrarySnapshot.Error}}
          <p class="summary">Last library query error: {{.LibrarySnapshot.Error}}</p>
        {{end}}
      </section>

      <section class="card">
        <h2>Volumes</h2>
        {{if .VolumeEntries}}
          <div class="library-list">
            {{range .VolumeEntries}}
              <article class="library-item">
                <p class="library-title">
                  {{.Label}}
                  {{if .IsAvailable}}
                    <span class="pill pill-ok">mounted</span>
                  {{else}}
                    <span class="pill pill-offline">missing</span>
                  {{end}}
                </p>
                <p class="library-meta">uuid {{.VolumeUUID}}</p>
                <p class="library-meta">{{.MountPath}}</p>
                <p class="library-meta">last seen {{.LastSeenAt}}</p>
              </article>
            {{end}}
          </div>
        {{else if .LibrarySnapshot.Available}}
          <p class="summary">No indexed volumes yet.</p>
        {{else}}
          <p class="summary">Library database is unavailable.</p>
        {{end}}
      </section>

      <section class="card">
        <h2>Albums</h2>
        {{if .AlbumEntries}}
          <div class="library-list">
            {{range .AlbumEntries}}
              <article class="library-item">
                {{if .CoverThumbPath}}
                  <a href="{{.CoverThumbPath}}" class="library-cover-link" target="_blank" rel="noreferrer">
                    <img src="{{.CoverThumbPath}}" alt="{{.Title}} cover" class="library-cover-art">
                  </a>
                {{end}}
                <p class="library-title">{{.Title}}</p>
                <p class="library-meta">{{.AlbumArtist}} · {{.YearLabel}} · {{.TrackCount}} tracks · {{.DurationLabel}}</p>
                <p class="library-meta">{{.RootDirHint}}</p>
                <p class="library-meta">
                  thumb
                  {{if .CoverThumbPath}}
                    <a href="{{.CoverThumbPath}}" target="_blank" rel="noreferrer">{{.CoverThumbLabel}}</a>
                  {{else}}
                    {{.CoverThumbLabel}}
                  {{end}}
                </p>
              </article>
            {{end}}
          </div>
        {{else if .LibrarySnapshot.Available}}
          <p class="summary">No indexed albums yet.</p>
        {{else}}
          <p class="summary">Library database is unavailable.</p>
        {{end}}
      </section>

      <section class="card">
        <h2>Tracks</h2>
        {{if .TrackEntries}}
          <div class="library-list">
            {{range .TrackEntries}}
              <article class="library-item">
                <p class="library-title">{{.Title}}</p>
                <p class="library-meta">{{.Artist}} · {{.FormatLabel}} · {{.DurationLabel}}</p>
                <p class="library-meta">{{.RelativePath}}</p>
              </article>
            {{end}}
          </div>
        {{else if .LibrarySnapshot.Available}}
          <p class="summary">No indexed tracks yet.</p>
        {{else}}
          <p class="summary">Library database is unavailable.</p>
        {{end}}
      </section>
    </main>
  </body>
</html>
~~~

## `services/controld/web/templates/logs.html`

- bytes: 1984
- segment: 1/1

~~~html
<!doctype html>
<html lang="en">
  <head>
    <meta charset="utf-8">
    <meta name="viewport" content="width=device-width, initial-scale=1">
    <title>Lumelo logs</title>
    <link rel="stylesheet" href="/static/css/app.css">
  </head>
  <body>
    <main class="shell">
      <nav class="topnav">
        <a href="/" class="topnav-link {{if eq .CurrentPage "home"}}topnav-link-current{{end}}">Home</a>
        <a href="/library" class="topnav-link {{if eq .CurrentPage "library"}}topnav-link-current{{end}}">Library</a>
        <a href="/provisioning" class="topnav-link {{if eq .CurrentPage "provisioning"}}topnav-link-current{{end}}">Provisioning</a>
        <a href="/logs" class="topnav-link {{if eq .CurrentPage "logs"}}topnav-link-current{{end}}">Logs</a>
      </nav>

      <section class="card">
        <p class="eyebrow">Lumelo bring-up</p>
        <h1>System logs</h1>
        <p class="summary">
          This page reads the current boot journal on demand so you can copy the
          latest runtime logs during T4 bring-up without opening an SSH session.
        </p>
      </section>

      <section class="card">
        <h2>Copy Logs</h2>
        {{if .LogError}}
          <p class="banner banner-error">Log read error: {{.LogError}}</p>
        {{end}}
        <p class="summary">
          Showing the latest {{.Lines}} lines from <code>journalctl -b</code>.
          Use the plain-text view if you want the easiest copy/paste payload.
        </p>
        <div class="actions log-actions">
          <a href="{{.LogTextPath}}" class="button-link">Open plain text</a>
          <a href="/logs?lines=100" class="button-link button-link-secondary">100 lines</a>
          <a href="/logs?lines=300" class="button-link button-link-secondary">300 lines</a>
          <a href="/logs?lines=1000" class="button-link button-link-secondary">1000 lines</a>
        </div>
        <pre class="log-output">{{.LogText}}</pre>
      </section>
    </main>
  </body>
</html>
~~~

## `services/controld/web/templates/provisioning.html`

- bytes: 6272
- segment: 1/1

~~~html
<!doctype html>
<html lang="en">
  <head>
    <meta charset="utf-8">
    <meta name="viewport" content="width=device-width, initial-scale=1">
    <title>Lumelo provisioning</title>
    <link rel="stylesheet" href="/static/css/app.css">
  </head>
  <body>
    <main class="shell">
      <nav class="topnav">
        <a href="/" class="topnav-link {{if eq .CurrentPage "home"}}topnav-link-current{{end}}">Home</a>
        <a href="/library" class="topnav-link {{if eq .CurrentPage "library"}}topnav-link-current{{end}}">Library</a>
        <a href="/provisioning" class="topnav-link {{if eq .CurrentPage "provisioning"}}topnav-link-current{{end}}">Provisioning</a>
        <a href="/logs" class="topnav-link {{if eq .CurrentPage "logs"}}topnav-link-current{{end}}">Logs</a>
      </nav>

      <section class="card">
        <p class="eyebrow">Lumelo bring-up</p>
        <h1>Provisioning status</h1>
        <p class="summary">
          This page shows the last BLE to Wi-Fi provisioning state written by
          the T4 helper so bring-up can be inspected without opening a shell.
        </p>
        <p class="summary">
          While bring-up is in progress, this page auto-refreshes when the
          provisioning status file changes.
        </p>
      </section>

      <section class="card" data-provisioning-updated-at="{{.Provisioning.UpdatedAt}}">
        <h2>Current State</h2>
        {{if .Provisioning.Available}}
          <p class="status-line">
            Status:
            {{if eq .Provisioning.State "connected"}}
              <span class="pill pill-ok">connected</span>
            {{else if eq .Provisioning.State "failed"}}
              <span class="pill pill-offline">failed</span>
            {{else}}
              <span class="pill pill-current">{{if .Provisioning.State}}{{.Provisioning.State}}{{else}}unknown{{end}}</span>
            {{end}}
          </p>
        {{else}}
          <p class="status-line">
            Status:
            <span class="pill pill-offline">unavailable</span>
          </p>
        {{end}}

        <dl class="grid">
          <div>
            <dt>Message</dt>
            <dd>{{if .Provisioning.Message}}{{.Provisioning.Message}}{{else}}-{{end}}</dd>
          </div>
          <div>
            <dt>SSID</dt>
            <dd>{{if .Provisioning.SSID}}{{.Provisioning.SSID}}{{else}}-{{end}}</dd>
          </div>
          <div>
            <dt>IP</dt>
            <dd>{{if .Provisioning.IP}}{{.Provisioning.IP}}{{else}}-{{end}}</dd>
          </div>
          <div>
            <dt>Wi-Fi IP</dt>
            <dd>{{if .Provisioning.WiFiIP}}{{.Provisioning.WiFiIP}}{{else}}-{{end}}</dd>
          </div>
          <div>
            <dt>Wired IP</dt>
            <dd>{{if .Provisioning.WiredIP}}{{.Provisioning.WiredIP}}{{else}}-{{end}}</dd>
          </div>
          <div>
            <dt>All IPs</dt>
            <dd>{{if .Provisioning.AllIPs}}{{range $index, $ip := .Provisioning.AllIPs}}{{if $index}}, {{end}}{{$ip}}{{end}}{{else}}-{{end}}</dd>
          </div>
          <div>
            <dt>WebUI</dt>
            <dd>{{if .Provisioning.WebURL}}<a href="{{.Provisioning.WebURL}}">{{.Provisioning.WebURL}}</a>{{else}}-{{end}}</dd>
          </div>
          <div>
            <dt>Wi-Fi interface</dt>
            <dd>{{if .Provisioning.WiFiInterface}}{{.Provisioning.WiFiInterface}}{{else}}-{{end}}</dd>
          </div>
          <div>
            <dt>WPA unit</dt>
            <dd>{{if .Provisioning.WPAUnit}}{{.Provisioning.WPAUnit}}{{else}}-{{end}}</dd>
          </div>
          <div>
            <dt>Hostname</dt>
            <dd>{{if .Provisioning.Hostname}}{{.Provisioning.Hostname}}{{else}}-{{end}}</dd>
          </div>
          <div>
            <dt>Updated at</dt>
            <dd>{{if .Provisioning.UpdatedAt}}{{.Provisioning.UpdatedAt}}{{else}}-{{end}}</dd>
          </div>
          <div>
            <dt>Status file</dt>
            <dd>{{if .Provisioning.StatusPath}}{{.Provisioning.StatusPath}}{{else}}-{{end}}</dd>
          </div>
          <div>
            <dt>Error code</dt>
            <dd>{{if .Provisioning.ErrorCode}}{{.Provisioning.ErrorCode}}{{else}}-{{end}}</dd>
          </div>
          <div>
            <dt>DHCP wait window</dt>
            <dd>{{if .Provisioning.IPWaitSeconds}}{{.Provisioning.IPWaitSeconds}} seconds{{else}}-{{end}}</dd>
          </div>
        </dl>

        {{if .Provisioning.Error}}
          <p class="banner banner-error">Provisioning error: {{.Provisioning.Error}}</p>
        {{end}}
        {{if .Provisioning.ReadError}}
          <p class="banner banner-error">Status read error: {{.Provisioning.ReadError}}</p>
        {{end}}
        {{if .Provisioning.ApplyOutput}}
          <p class="summary">Apply output: {{.Provisioning.ApplyOutput}}</p>
        {{end}}
        {{if .Provisioning.DiagnosticHint}}
          <p class="summary">Diagnostic hint: {{.Provisioning.DiagnosticHint}}</p>
        {{end}}

        <div class="actions log-actions">
          <a href="/provisioning-status" class="button-link">Open JSON</a>
          <a href="/healthz" class="button-link button-link-secondary">Healthz</a>
          <a href="/logs?lines=300" class="button-link button-link-secondary">Logs</a>
        </div>
      </section>

      <section class="card">
        <h2>Raw JSON</h2>
        <pre class="log-output">{{.RawJSON}}</pre>
      </section>
    </main>
    <script>
      (() => {
        const card = document.querySelector("[data-provisioning-updated-at]");
        if (!card || !window.fetch) {
          return;
        }

        let latestUpdatedAt = card.getAttribute("data-provisioning-updated-at") || "";
        const poll = async () => {
          try {
            const response = await fetch("/provisioning-status", { cache: "no-store" });
            if (!response.ok) {
              return;
            }
            const payload = await response.json();
            if (payload && payload.updated_at && payload.updated_at !== latestUpdatedAt) {
              window.location.reload();
            }
          } catch (error) {
            // Ignore transient refresh errors during bring-up.
          }
        };

        window.setInterval(poll, 3000);
      })();
    </script>
  </body>
</html>
~~~

## `services/rust/Cargo.lock`

- bytes: 17019
- segment: 1/1

~~~text
# This file is automatically @generated by Cargo.
# It is not intended for manual editing.
version = 4

[[package]]
name = "adler2"
version = "2.0.1"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "320119579fcad9c21884f5c4861d16174d0e06250625266f50fe6898340abefa"

[[package]]
name = "ahash"
version = "0.8.12"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "5a15f179cd60c4584b8a8c596927aadc462e27f2ca70c04e0071964a73ba7a75"
dependencies = [
 "cfg-if",
 "once_cell",
 "version_check",
 "zerocopy",
]

[[package]]
name = "arrayvec"
version = "0.7.6"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "7c02d123df017efcdfbd739ef81735b36c5ba83ec3c59c80a9d7ecc718f92e50"

[[package]]
name = "artwork-cache"
version = "0.1.0"

[[package]]
name = "autocfg"
version = "1.5.0"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "c08606f8c3cbf4ce6ec8e28fb0014a2c086708fe954eaa885384a6165172e7e8"

[[package]]
name = "bitflags"
version = "1.3.2"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "bef38d45163c2f1dde094a7dfd33ccf595c92905c8f8f4fdc18d06fb1037718a"

[[package]]
name = "bitflags"
version = "2.11.0"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "843867be96c8daad0d758b57df9392b6d8d271134fce549de6ce169ff98a92af"

[[package]]
name = "bytemuck"
version = "1.25.0"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "c8efb64bd706a16a1bdde310ae86b351e4d21550d98d056f22f8a7f7a2183fec"

[[package]]
name = "byteorder"
version = "1.5.0"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "1fd0f2584146f6f2ef48085050886acf353beff7305ebd1ae69500e27c67f64b"

[[package]]
name = "byteorder-lite"
version = "0.1.0"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "8f1fe948ff07f4bd06c30984e69f5b4899c516a3ef74f34df92a2df2ab535495"

[[package]]
name = "cc"
version = "1.2.59"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "b7a4d3ec6524d28a329fc53654bbadc9bdd7b0431f5d65f1a56ffb28a1ee5283"
dependencies = [
 "find-msvc-tools",
 "shlex",
]

[[package]]
name = "cfg-if"
version = "1.0.4"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "9330f8b2ff13f34540b44e946ef35111825727b38d33286ef986142615121801"

[[package]]
name = "crc32fast"
version = "1.5.0"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "9481c1c90cbf2ac953f07c8d4a58aa3945c425b7185c9154d67a65e4230da511"
dependencies = [
 "cfg-if",
]

[[package]]
name = "data-encoding"
version = "2.10.0"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "d7a1e2f27636f116493b8b860f5546edb47c8d8f8ea73e1d2a20be88e28d1fea"

[[package]]
name = "encoding_rs"
version = "0.8.35"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "75030f3c4f45dafd7586dd6780965a8c7e8e285a5ecb86713e63a79c5b2766f3"
dependencies = [
 "cfg-if",
]

[[package]]
name = "extended"
version = "0.1.0"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "af9673d8203fcb076b19dfd17e38b3d4ae9f44959416ea532ce72415a6020365"

[[package]]
name = "fallible-iterator"
version = "0.3.0"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "2acce4a10f12dc2fb14a218589d4f1f62ef011b2d0cc4b3cb1bba8e94da14649"

[[package]]
name = "fallible-streaming-iterator"
version = "0.1.9"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "7360491ce676a36bf9bb3c56c1aa791658183a54d2744120f27285738d90465a"

[[package]]
name = "fdeflate"
version = "0.3.7"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "1e6853b52649d4ac5c0bd02320cddc5ba956bdb407c4b75a2c6b75bf51500f8c"
dependencies = [
 "simd-adler32",
]

[[package]]
name = "find-msvc-tools"
version = "0.1.9"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "5baebc0774151f905a1a2cc41989300b1e6fbb29aff0ceffa1064fdd3088d582"

[[package]]
name = "flate2"
version = "1.1.9"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "843fba2746e448b37e26a819579957415c8cef339bf08564fe8b7ddbd959573c"
dependencies = [
 "crc32fast",
 "miniz_oxide",
]

[[package]]
name = "hashbrown"
version = "0.14.5"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "e5274423e17b7c9fc20b6e7e208532f9b19825d82dfd615708b70edd83df41f1"
dependencies = [
 "ahash",
]

[[package]]
name = "hashlink"
version = "0.9.1"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "6ba4ff7128dee98c7dc9794b6a411377e1404dba1c97deb8d1a55297bd25d8af"
dependencies = [
 "hashbrown",
]

[[package]]
name = "image"
version = "0.25.10"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "85ab80394333c02fe689eaf900ab500fbd0c2213da414687ebf995a65d5a6104"
dependencies = [
 "bytemuck",
 "byteorder-lite",
 "moxcms",
 "num-traits",
 "png",
 "zune-core",
 "zune-jpeg",
]

[[package]]
name = "ipc-proto"
version = "0.1.0"
dependencies = [
 "serde",
 "serde_json",
]

[[package]]
name = "itoa"
version = "1.0.18"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "8f42a60cbdf9a97f5d2305f08a87dc4e09308d1276d28c869c684d7777685682"

[[package]]
name = "lazy_static"
version = "1.5.0"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "bbd2bcb4c963f2ddae06a2efc7e9f3591312473c50c6685e1f298068316e66fe"

[[package]]
name = "libsqlite3-sys"
version = "0.30.1"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "2e99fb7a497b1e3339bc746195567ed8d3e24945ecd636e3619d20b9de9e9149"
dependencies = [
 "cc",
 "pkg-config",
 "vcpkg",
]

[[package]]
name = "lofty"
version = "0.21.1"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "c8bc4717ff10833a623b009e9254ae8667c7a59edc3cfb01c37aeeef4b6d54a7"
dependencies = [
 "byteorder",
 "data-encoding",
 "flate2",
 "lofty_attr",
 "log",
 "ogg_pager",
 "paste",
]

[[package]]
name = "lofty_attr"
version = "0.11.1"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "ed9983e64b2358522f745c1251924e3ab7252d55637e80f6a0a3de642d6a9efc"
dependencies = [
 "proc-macro2",
 "quote",
 "syn",
]

[[package]]
name = "log"
version = "0.4.29"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "5e5032e24019045c762d3c0f28f5b6b8bbf38563a65908389bf7978758920897"

[[package]]
name = "media-indexd"
version = "0.1.0"
dependencies = [
 "artwork-cache",
 "image",
 "ipc-proto",
 "lofty",
 "rusqlite",
]

[[package]]
name = "media-model"
version = "0.1.0"
dependencies = [
 "serde",
]

[[package]]
name = "memchr"
version = "2.8.0"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "f8ca58f447f06ed17d5fc4043ce1b10dd205e060fb3ce5b979b8ed8e59ff3f79"

[[package]]
name = "miniz_oxide"
version = "0.8.9"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "1fa76a2c86f704bdb222d66965fb3d63269ce38518b83cb0575fca855ebb6316"
dependencies = [
 "adler2",
 "simd-adler32",
]

[[package]]
name = "moxcms"
version = "0.8.1"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "bb85c154ba489f01b25c0d36ae69a87e4a1c73a72631fc6c0eb6dde34a73e44b"
dependencies = [
 "num-traits",
 "pxfm",
]

[[package]]
name = "num-traits"
version = "0.2.19"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "071dfc062690e90b734c0b2273ce72ad0ffa95f0c74596bc250dcfd960262841"
dependencies = [
 "autocfg",
]

[[package]]
name = "ogg_pager"
version = "0.6.1"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "87b0bef808533c5890ab77279538212efdbbbd9aa4ef1ccdfcfbf77a42f7e6fa"
dependencies = [
 "byteorder",
]

[[package]]
name = "once_cell"
version = "1.21.4"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "9f7c3e4beb33f85d45ae3e3a1792185706c8e16d043238c593331cc7cd313b50"

[[package]]
name = "paste"
version = "1.0.15"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "57c0d7b74b563b49d38dae00a0c37d4d6de9b432382b2892f0574ddcae73fd0a"

[[package]]
name = "pkg-config"
version = "0.3.32"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "7edddbd0b52d732b21ad9a5fab5c704c14cd949e5e9a1ec5929a24fded1b904c"

[[package]]
name = "playbackd"
version = "0.1.0"
dependencies = [
 "ipc-proto",
 "media-model",
 "rusqlite",
 "serde",
 "serde_json",
 "symphonia",
]

[[package]]
name = "png"
version = "0.18.1"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "60769b8b31b2a9f263dae2776c37b1b28ae246943cf719eb6946a1db05128a61"
dependencies = [
 "bitflags 2.11.0",
 "crc32fast",
 "fdeflate",
 "flate2",
 "miniz_oxide",
]

[[package]]
name = "proc-macro2"
version = "1.0.106"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "8fd00f0bb2e90d81d1044c2b32617f68fcb9fa3bb7640c23e9c748e53fb30934"
dependencies = [
 "unicode-ident",
]

[[package]]
name = "pxfm"
version = "0.1.28"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "b5a041e753da8b807c9255f28de81879c78c876392ff2469cde94799b2896b9d"

[[package]]
name = "quote"
version = "1.0.45"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "41f2619966050689382d2b44f664f4bc593e129785a36d6ee376ddf37259b924"
dependencies = [
 "proc-macro2",
]

[[package]]
name = "rusqlite"
version = "0.32.1"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "7753b721174eb8ff87a9a0e799e2d7bc3749323e773db92e0984debb00019d6e"
dependencies = [
 "bitflags 2.11.0",
 "fallible-iterator",
 "fallible-streaming-iterator",
 "hashlink",
 "libsqlite3-sys",
 "smallvec",
]

[[package]]
name = "serde"
version = "1.0.228"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "9a8e94ea7f378bd32cbbd37198a4a91436180c5bb472411e48b5ec2e2124ae9e"
dependencies = [
 "serde_core",
 "serde_derive",
]

[[package]]
name = "serde_core"
version = "1.0.228"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "41d385c7d4ca58e59fc732af25c3983b67ac852c1a25000afe1175de458b67ad"
dependencies = [
 "serde_derive",
]

[[package]]
name = "serde_derive"
version = "1.0.228"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "d540f220d3187173da220f885ab66608367b6574e925011a9353e4badda91d79"
dependencies = [
 "proc-macro2",
 "quote",
 "syn",
]

[[package]]
name = "serde_json"
version = "1.0.149"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "83fc039473c5595ace860d8c4fafa220ff474b3fc6bfdb4293327f1a37e94d86"
dependencies = [
 "itoa",
 "memchr",
 "serde",
 "serde_core",
 "zmij",
]

[[package]]
name = "sessiond"
version = "0.1.0"
dependencies = [
 "ipc-proto",
]

[[package]]
name = "shlex"
version = "1.3.0"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "0fda2ff0d084019ba4d7c6f371c95d8fd75ce3524c3cb8fb653a3023f6323e64"

[[package]]
name = "simd-adler32"
version = "0.3.9"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "703d5c7ef118737c72f1af64ad2f6f8c5e1921f818cdcb97b8fe6fc69bf66214"

[[package]]
name = "smallvec"
version = "1.15.1"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "67b1b7a3b5fe4f1376887184045fcf45c69e92af734b7aaddc05fb777b6fbd03"

[[package]]
name = "symphonia"
version = "0.5.5"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "5773a4c030a19d9bfaa090f49746ff35c75dfddfa700df7a5939d5e076a57039"
dependencies = [
 "lazy_static",
 "symphonia-bundle-flac",
 "symphonia-bundle-mp3",
 "symphonia-codec-aac",
 "symphonia-codec-pcm",
 "symphonia-codec-vorbis",
 "symphonia-core",
 "symphonia-format-isomp4",
 "symphonia-format-ogg",
 "symphonia-format-riff",
 "symphonia-metadata",
]

[[package]]
name = "symphonia-bundle-flac"
version = "0.5.5"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "c91565e180aea25d9b80a910c546802526ffd0072d0b8974e3ebe59b686c9976"
dependencies = [
 "log",
 "symphonia-core",
 "symphonia-metadata",
 "symphonia-utils-xiph",
]

[[package]]
name = "symphonia-bundle-mp3"
version = "0.5.5"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "4872dd6bb56bf5eac799e3e957aa1981086c3e613b27e0ac23b176054f7c57ed"
dependencies = [
 "lazy_static",
 "log",
 "symphonia-core",
 "symphonia-metadata",
]

[[package]]
name = "symphonia-codec-aac"
version = "0.5.5"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "4c263845aa86881416849c1729a54c7f55164f8b96111dba59de46849e73a790"
dependencies = [
 "lazy_static",
 "log",
 "symphonia-core",
]

[[package]]
name = "symphonia-codec-pcm"
version = "0.5.5"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "4e89d716c01541ad3ebe7c91ce4c8d38a7cf266a3f7b2f090b108fb0cb031d95"
dependencies = [
 "log",
 "symphonia-core",
]

[[package]]
name = "symphonia-codec-vorbis"
version = "0.5.5"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "f025837c309cd69ffef572750b4a2257b59552c5399a5e49707cc5b1b85d1c73"
dependencies = [
 "log",
 "symphonia-core",
 "symphonia-utils-xiph",
]

[[package]]
name = "symphonia-core"
version = "0.5.5"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "ea00cc4f79b7f6bb7ff87eddc065a1066f3a43fe1875979056672c9ef948c2af"
dependencies = [
 "arrayvec",
 "bitflags 1.3.2",
 "bytemuck",
 "lazy_static",
 "log",
]

[[package]]
name = "symphonia-format-isomp4"
version = "0.5.5"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "243739585d11f81daf8dac8d9f3d18cc7898f6c09a259675fc364b382c30e0a5"
dependencies = [
 "encoding_rs",
 "log",
 "symphonia-core",
 "symphonia-metadata",
 "symphonia-utils-xiph",
]

[[package]]
name = "symphonia-format-ogg"
version = "0.5.5"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "2b4955c67c1ed3aa8ae8428d04ca8397fbef6a19b2b051e73b5da8b1435639cb"
dependencies = [
 "log",
 "symphonia-core",
 "symphonia-metadata",
 "symphonia-utils-xiph",
]

[[package]]
name = "symphonia-format-riff"
version = "0.5.5"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "c2d7c3df0e7d94efb68401d81906eae73c02b40d5ec1a141962c592d0f11a96f"
dependencies = [
 "extended",
 "log",
 "symphonia-core",
 "symphonia-metadata",
]

[[package]]
name = "symphonia-metadata"
version = "0.5.5"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "36306ff42b9ffe6e5afc99d49e121e0bd62fe79b9db7b9681d48e29fa19e6b16"
dependencies = [
 "encoding_rs",
 "lazy_static",
 "log",
 "symphonia-core",
]

[[package]]
name = "symphonia-utils-xiph"
version = "0.5.5"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "ee27c85ab799a338446b68eec77abf42e1a6f1bb490656e121c6e27bfbab9f16"
dependencies = [
 "symphonia-core",
 "symphonia-metadata",
]

[[package]]
name = "syn"
version = "2.0.117"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "e665b8803e7b1d2a727f4023456bbbbe74da67099c585258af0ad9c5013b9b99"
dependencies = [
 "proc-macro2",
 "quote",
 "unicode-ident",
]

[[package]]
name = "unicode-ident"
version = "1.0.24"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "e6e4313cd5fcd3dad5cafa179702e2b244f760991f45397d14d4ebf38247da75"

[[package]]
name = "vcpkg"
version = "0.2.15"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "accd4ea62f7bb7a82fe23066fb0957d48ef677f6eeb8215f372f52e48bb32426"

[[package]]
name = "version_check"
version = "0.9.5"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "0b928f33d975fc6ad9f86c8f283853ad26bdd5b10b7f1542aa2fa15e2289105a"

[[package]]
name = "zerocopy"
version = "0.8.48"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "eed437bf9d6692032087e337407a86f04cd8d6a16a37199ed57949d415bd68e9"
dependencies = [
 "zerocopy-derive",
]

[[package]]
name = "zerocopy-derive"
version = "0.8.48"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "70e3cd084b1788766f53af483dd21f93881ff30d7320490ec3ef7526d203bad4"
dependencies = [
 "proc-macro2",
 "quote",
 "syn",
]

[[package]]
name = "zmij"
version = "1.0.21"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "b8848ee67ecc8aedbaf3e4122217aff892639231befc6a1b58d29fff4c2cabaa"

[[package]]
name = "zune-core"
version = "0.5.1"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "cb8a0807f7c01457d0379ba880ba6322660448ddebc890ce29bb64da71fb40f9"

[[package]]
name = "zune-jpeg"
version = "0.5.15"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "27bc9d5b815bc103f142aa054f561d9187d191692ec7c2d1e2b4737f8dbd7296"
dependencies = [
 "zune-core",
]
~~~

## `services/rust/Cargo.toml`

- bytes: 273
- segment: 1/1

~~~toml
[workspace]
members = [
  "crates/playbackd",
  "crates/sessiond",
  "crates/media-indexd",
  "crates/ipc-proto",
  "crates/media-model",
  "crates/artwork-cache",
]
resolver = "2"

[workspace.package]
edition = "2021"
version = "0.1.0"
license = "MIT"
authors = ["Codex"]
~~~

## `services/rust/README.md`

- bytes: 362
- segment: 1/1

~~~md
# Rust Workspace

This workspace contains the playback-side services and shared crates for V1:

- `playbackd`: playback state authority
- `sessiond`: quiet-mode controller
- `media-indexd`: on-demand indexing worker
- `ipc-proto`: command/event boundary types
- `media-model`: queue and library-facing shared models
- `artwork-cache`: artwork cache path helpers
~~~

## `services/rust/crates/artwork-cache/Cargo.toml`

- bytes: 160
- segment: 1/1

~~~toml
[package]
name = "artwork-cache"
version.workspace = true
edition.workspace = true
license.workspace = true
authors.workspace = true

[lib]
path = "src/lib.rs"
~~~

## `services/rust/crates/artwork-cache/src/lib.rs`

- bytes: 1658
- segment: 1/1

~~~rust
use std::path::PathBuf;

pub fn bucket_segments(cover_ref_id: &str) -> (String, String) {
    let mut padded = cover_ref_id.to_owned();
    while padded.len() < 4 {
        padded.push('0');
    }

    (padded[0..2].to_string(), padded[2..4].to_string())
}

pub fn thumb_path(root: &str, cover_ref_id: &str) -> PathBuf {
    let (first, second) = bucket_segments(cover_ref_id);

    PathBuf::from(root)
        .join("thumb")
        .join("320")
        .join(first)
        .join(second)
        .join(format!("{cover_ref_id}.jpg"))
}

pub fn source_path(root: &str, cover_ref_id: &str, extension: &str) -> PathBuf {
    let (first, second) = bucket_segments(cover_ref_id);
    let extension = extension.trim_start_matches('.').to_ascii_lowercase();

    PathBuf::from(root)
        .join("source")
        .join(first)
        .join(second)
        .join(format!("{cover_ref_id}.{extension}"))
}

#[cfg(test)]
mod tests {
    use super::{bucket_segments, source_path, thumb_path};

    #[test]
    fn buckets_are_zero_padded() {
        assert_eq!(bucket_segments("a"), ("a0".to_string(), "00".to_string()));
    }

    #[test]
    fn thumb_path_uses_expected_layout() {
        let path = thumb_path("/var/cache/lumelo/artwork", "a1b2c3");
        assert_eq!(
            path.to_string_lossy(),
            "/var/cache/lumelo/artwork/thumb/320/a1/b2/a1b2c3.jpg"
        );
    }

    #[test]
    fn source_path_uses_expected_layout() {
        let path = source_path("/var/cache/lumelo/artwork", "a1b2c3", "PNG");
        assert_eq!(
            path.to_string_lossy(),
            "/var/cache/lumelo/artwork/source/a1/b2/a1b2c3.png"
        );
    }
}
~~~

## `services/rust/crates/ipc-proto/Cargo.toml`

- bytes: 238
- segment: 1/1

~~~toml
[package]
name = "ipc-proto"
version.workspace = true
edition.workspace = true
license.workspace = true
authors.workspace = true

[lib]
path = "src/lib.rs"

[dependencies]
serde = { version = "1", features = ["derive"] }
serde_json = "1"
~~~

