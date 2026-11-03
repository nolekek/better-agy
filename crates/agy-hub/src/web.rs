//! Web frontend templates and SPA dashboard embedded assets

pub const LOGIN_HTML: &str = r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Antigravity Multi-User Server | Login</title>
    <style>
        :root {
            --bg-color: #0f172a;
            --card-bg: #1e293b;
            --accent: #38bdf8;
            --accent-hover: #0284c7;
            --text-main: #f8fafc;
            --text-muted: #94a3b8;
            --error: #ef4444;
            --border: #334155;
        }
        * { box-sizing: border-box; margin: 0; padding: 0; font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, Helvetica, Arial, sans-serif; }
        body { background: var(--bg-color); color: var(--text-main); display: flex; align-items: center; justify-content: center; min-height: 100vh; }
        .login-card { background: var(--card-bg); border: 1px solid var(--border); border-radius: 12px; padding: 2.5rem; width: 100%; max-width: 400px; box-shadow: 0 10px 25px rgba(0,0,0,0.5); }
        .logo { font-size: 1.5rem; font-weight: 700; color: var(--accent); margin-bottom: 0.5rem; display: flex; align-items: center; gap: 0.5rem; }
        .subtitle { color: var(--text-muted); font-size: 0.875rem; margin-bottom: 2rem; }
        .form-group { margin-bottom: 1.25rem; }
        label { display: block; font-size: 0.75rem; text-transform: uppercase; letter-spacing: 0.05em; color: var(--text-muted); margin-bottom: 0.5rem; }
        input { width: 100%; padding: 0.75rem 1rem; border-radius: 6px; border: 1px solid var(--border); background: #0b1120; color: var(--text-main); font-size: 0.95rem; }
        input:focus { outline: none; border-color: var(--accent); }
        button { width: 100%; padding: 0.75rem; border-radius: 6px; border: none; background: var(--accent); color: #0b1120; font-weight: 600; font-size: 1rem; cursor: pointer; transition: background 0.2s; margin-top: 1rem; }
        button:hover { background: var(--accent-hover); }
        .error-box { background: rgba(239,68,68,0.15); border: 1px solid var(--error); color: #fca5a5; padding: 0.75rem; border-radius: 6px; font-size: 0.875rem; margin-bottom: 1rem; display: none; }
    </style>
</head>
<body>
    <div class="login-card">
        <div class="logo">⚡ Antigravity Server</div>
        <div class="subtitle">Multi-User Development Portal (PAM Protected)</div>
        <div id="error-box" class="error-box"></div>
        <form id="login-form">
            <div class="form-group">
                <label for="username">Linux Username</label>
                <input type="text" id="username" name="username" required autocomplete="username" autofocus>
            </div>
            <div class="form-group">
                <label for="password">Password</label>
                <input type="password" id="password" name="password" required autocomplete="current-password">
            </div>
            <button type="submit">Sign In</button>
        </form>
    </div>
    <script>
        document.getElementById('login-form').addEventListener('submit', async (e) => {
            e.preventDefault();
            const errorBox = document.getElementById('error-box');
            errorBox.style.display = 'none';
            const username = document.getElementById('username').value;
            const password = document.getElementById('password').value;

            try {
                const res = await fetch('/api/auth/login', {
                    method: 'POST',
                    headers: { 'Content-Type': 'application/json' },
                    body: JSON.stringify({ username, password })
                });
                const data = await res.json();
                if (res.ok && data.ok) {
                    window.location.href = data.redirect || '/dashboard';
                } else {
                    errorBox.textContent = data.error || 'Authentication failed';
                    errorBox.style.display = 'block';
                }
            } catch (err) {
                errorBox.textContent = 'Server connection error';
                errorBox.style.display = 'block';
            }
        });
    </script>
</body>
</html>
"#;

pub const DASHBOARD_HTML: &str = r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Antigravity Multi-User Server | Management Dashboard</title>
    <style>
        :root {
            --bg-color: #090d16;
            --card-bg: #131c2e;
            --border: #1e293b;
            --accent: #38bdf8;
            --accent-hover: #0ea5e9;
            --success: #10b981;
            --warning: #f59e0b;
            --error: #ef4444;
            --text-main: #f8fafc;
            --text-muted: #94a3b8;
        }
        * { box-sizing: border-box; margin: 0; padding: 0; font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, Helvetica, Arial, sans-serif; }
        body { background: var(--bg-color); color: var(--text-main); padding: 2rem; }
        .container { max-width: 1200px; margin: 0 auto; }
        header { display: flex; justify-content: space-between; align-items: center; border-bottom: 1px solid var(--border); padding-bottom: 1.5rem; margin-bottom: 2rem; }
        .brand { font-size: 1.5rem; font-weight: 700; color: var(--accent); display: flex; align-items: center; gap: 0.5rem; }
        .user-panel { display: flex; align-items: center; gap: 1rem; }
        .user-badge { background: #1e293b; padding: 0.4rem 0.8rem; border-radius: 9999px; font-size: 0.875rem; border: 1px solid #334155; }
        .btn { padding: 0.5rem 1rem; border-radius: 6px; font-weight: 600; font-size: 0.875rem; cursor: pointer; border: none; transition: 0.2s; text-decoration: none; display: inline-flex; align-items: center; justify-content: center; }
        .btn-primary { background: var(--accent); color: #090d16; }
        .btn-primary:hover { background: var(--accent-hover); }
        .btn-secondary { background: #1e293b; color: var(--text-main); border: 1px solid var(--border); }
        .btn-secondary:hover { background: #334155; }
        .btn-danger { background: rgba(239,68,68,0.2); color: #fca5a5; border: 1px solid var(--error); }
        .grid { display: grid; grid-template-columns: repeat(auto-fit, minmax(350px, 1fr)); gap: 1.5rem; margin-bottom: 2rem; }
        .card { background: var(--card-bg); border: 1px solid var(--border); border-radius: 12px; padding: 1.5rem; }
        .card-title { font-size: 1.1rem; font-weight: 600; margin-bottom: 1rem; display: flex; justify-content: space-between; align-items: center; }
        .status-badge { display: inline-block; padding: 0.25rem 0.5rem; border-radius: 4px; font-size: 0.75rem; font-weight: 700; text-transform: uppercase; }
        .status-active { background: rgba(16,185,129,0.2); color: var(--success); border: 1px solid var(--success); }
        .status-inactive { background: rgba(148,163,184,0.2); color: var(--text-muted); border: 1px solid var(--text-muted); }
        .telemetry-row { margin-bottom: 1rem; }
        .progress-bar-bg { background: #1e293b; border-radius: 9999px; height: 10px; overflow: hidden; margin-top: 0.4rem; }
        .progress-bar-fill { background: var(--accent); height: 100%; border-radius: 9999px; transition: width 0.3s; }
        .presets-grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(130px, 1fr)); gap: 0.75rem; margin-top: 1rem; }
        .preset-card { background: #0f172a; border: 1px solid var(--border); border-radius: 8px; padding: 0.75rem; text-align: center; cursor: pointer; transition: 0.2s; }
        .preset-card:hover { border-color: var(--accent); }
        .preset-card.active { border-color: var(--accent); background: rgba(56,189,248,0.1); }
        .preset-name { font-weight: 600; font-size: 0.85rem; margin-bottom: 0.25rem; }
        .controls-group { display: flex; gap: 0.5rem; margin-top: 1rem; }
    </style>
</head>
<body>
    <div class="container">
        <header>
            <div class="brand">⚡ Antigravity Control Center</div>
            <div class="user-panel">
                <span class="user-badge" id="user-badge">Loading user...</span>
                <a href="/ide/" class="btn btn-primary" target="_blank">Launch Web IDE</a>
                <button class="btn btn-secondary" id="logout-btn">Logout</button>
            </div>
        </header>

        <div class="grid">
            <div class="card">
                <div class="card-title">
                    <span>Web IDE Service</span>
                    <span id="service-status-badge" class="status-badge status-inactive">UNKNOWN</span>
                </div>
                <p style="font-size: 0.875rem; color: var(--text-muted);" id="service-details">Querying systemd user service status...</p>
                <div class="controls-group">
                    <button class="btn btn-primary" id="btn-start">Start</button>
                    <button class="btn btn-secondary" id="btn-restart">Restart</button>
                    <button class="btn btn-danger" id="btn-stop">Stop</button>
                </div>
            </div>

            <div class="card">
                <div class="card-title">
                    <span>Quota & Failover Pool</span>
                    <span class="status-badge status-active">ACTIVE (25s Guard)</span>
                </div>
                <div class="telemetry-row">
                    <div style="display:flex; justify-content:space-between; font-size:0.875rem;">
                        <span>Rolling 5-Hour Usage</span>
                        <span id="quota-5h-text">24%</span>
                    </div>
                    <div class="progress-bar-bg"><div id="quota-5h-bar" class="progress-bar-fill" style="width: 24%;"></div></div>
                </div>
                <div class="telemetry-row">
                    <div style="display:flex; justify-content:space-between; font-size:0.875rem;">
                        <span>Weekly Limit Usage</span>
                        <span id="quota-weekly-text">15%</span>
                    </div>
                    <div class="progress-bar-bg"><div id="quota-weekly-bar" class="progress-bar-fill" style="width: 15%;"></div></div>
                </div>
                <p style="font-size: 0.8rem; color: var(--text-muted); margin-top: 0.5rem;" id="active-slot-info">Slot 1/4: Active</p>
            </div>
        </div>

        <div class="card">
            <div class="card-title">
                <span>Domain Preset Switcher (Symlink Projection)</span>
                <span id="active-preset-badge" class="status-badge status-active">ACTIVE: web</span>
            </div>
            <p style="font-size: 0.875rem; color: var(--text-muted);">Switch skill projections in workspace <code id="workspace-path">~/.gemini/skills</code> with sub-5ms atomic symlinks and 5 immutable meta-skills.</p>
            <div class="presets-grid" id="presets-grid">
                <!-- 16 presets injected here -->
            </div>
        </div>
    </div>

    <script>
        const PRESETS = [
            "web", "backend", "devops", "appsec", "qa", "ai-agents",
            "web-research", "reverse-engineering", "product", "orchestrator",
            "systems", "vst3", "hardware", "media", "meta-dev", "minimal"
        ];
        let currentPreset = "web";

        async function init() {
            try {
                const meRes = await fetch('/api/hub/me');
                if (!meRes.ok) { window.location.href = '/login'; return; }
                const me = await meRes.json();
                document.getElementById('user-badge').textContent = `${me.username} (UID ${me.uid})`;

                renderPresets();
                refreshStatus();
                setInterval(refreshStatus, 5000);
            } catch (err) {
                console.error(err);
            }
        }

        function renderPresets() {
            const grid = document.getElementById('presets-grid');
            grid.innerHTML = '';
            PRESETS.forEach(p => {
                const div = document.createElement('div');
                div.className = `preset-card ${p === currentPreset ? 'active' : ''}`;
                div.innerHTML = `<div class="preset-name">${p}</div><div style="font-size:0.7rem; color:#94a3b8;">${p === currentPreset ? 'Active' : 'Switch'}</div>`;
                div.onclick = () => switchPreset(p);
                grid.appendChild(div);
            });
        }

        async function refreshStatus() {
            try {
                const res = await fetch('/api/hub/status');
                if (res.ok) {
                    const data = await res.json();
                    const badge = document.getElementById('service-status-badge');
                    const details = document.getElementById('service-details');
                    if (data.service && data.service.active_state === 'active') {
                        badge.className = 'status-badge status-active';
                        badge.textContent = 'RUNNING';
                        details.textContent = `Socket: ${data.service.socket_path} | Uptime: ${data.service.uptime_seconds}s`;
                    } else {
                        badge.className = 'status-badge status-inactive';
                        badge.textContent = 'STOPPED';
                        details.textContent = `Service inactive. Click Start to launch Web IDE.`;
                    }
                    if (data.active_preset) {
                        currentPreset = data.active_preset;
                        document.getElementById('active-preset-badge').textContent = `ACTIVE: ${currentPreset}`;
                        renderPresets();
                    }
                }
            } catch (err) {
                console.error("Status fetch error", err);
            }
        }

        async function switchPreset(presetName) {
            try {
                const res = await fetch('/api/hub/presets/switch', {
                    method: 'POST',
                    headers: { 'Content-Type': 'application/json' },
                    body: JSON.stringify({ preset: presetName })
                });
                if (res.ok) {
                    currentPreset = presetName;
                    document.getElementById('active-preset-badge').textContent = `ACTIVE: ${currentPreset}`;
                    renderPresets();
                }
            } catch (err) {
                alert("Failed to switch preset: " + err);
            }
        }

        document.getElementById('btn-start').onclick = async () => {
            await fetch('/api/hub/ide/start', { method: 'POST' });
            refreshStatus();
        };
        document.getElementById('btn-restart').onclick = async () => {
            await fetch('/api/hub/ide/restart', { method: 'POST' });
            refreshStatus();
        };
        document.getElementById('btn-stop').onclick = async () => {
            await fetch('/api/hub/ide/stop', { method: 'POST' });
            refreshStatus();
        };
        document.getElementById('logout-btn').onclick = async () => {
            await fetch('/api/auth/logout', { method: 'POST' });
            window.location.href = '/login';
        };

        init();
    </script>
</body>
</html>
"#;
