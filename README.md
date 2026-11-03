# better-agy: Antigravity Linux Multi-User Server

[![CI](https://github.com/nolekek/better-agy/actions/workflows/ci.yml/badge.svg)](https://github.com/nolekek/better-agy/actions/workflows/ci.yml)
[![Release](https://img.shields.io/github/v/release/nolekek/better-agy?style=flat&color=blue)](https://github.com/nolekek/better-agy/releases)
[![License: MIT](https://img.shields.io/badge/License-MIT-green.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/Rust-1.80%2B-orange.svg)](https://www.rust-lang.org/)
[![Target: Linux](https://img.shields.io/badge/Target-Ubuntu%20%7C%20Debian%20%7C%20RHEL-purple.svg)](https://ubuntu.com/)

A production-grade, multi-user Linux server environment for the **Google Antigravity Web IDE** (`antigravity-ide serve-web`). Designed to host 2+ concurrent developers on a single Linux host with automated TLS, strict POSIX user isolation, centralized PAM authentication, session-managed reverse proxying, resilient Google API quota failover via a dedicated proxy vault, and atomic symlink projection for skill presets.

---

## Architecture Overview

```
                          [ Internet / Browser ]
                                     │
                                     ▼ HTTPS (Port 443 / 80)
                       ┌───────────────────────────┐
                       │    Caddy Ingress          │
                       │   - Automated ACME TLS    │
                       │   - WebSocket Upgrade     │
                       │   - zstd/gzip compression │
                       └─────────────┬─────────────┘
                                     │ HTTP (127.0.0.1:8080)
                                     ▼
                       ┌───────────────────────────┐
                       │    agy-hub Service        │
                       │   - PAM Auth & JWT Cookie │
                       │   - Systemd D-Bus Control │
                       │   - Dynamic Proxy & WS    │
                       │   - Web Dashboard         │
                       └─────────────┬─────────────┘
                                     │ Unix Domain Sockets
                      ┌──────────────┴──────────────┐
                      ▼                             ▼
       /run/user/1001/antigravity.sock  /run/user/1002/antigravity.sock
       ┌─────────────────────────────┐ ┌─────────────────────────────┐
       │ User: dev1 (UID 1001)       │ │ User: dev2 (UID 1002)       │
       │ systemd user unit:          │ │ systemd user unit:          │
       │ antigravity-web.service     │ │ antigravity-web.service     │
       │ HTTPS_PROXY=127.0.0.1:51128 │ │ HTTPS_PROXY=127.0.0.1:51128 │
       │ Workspace: ~dev1/workspace  │ │ Workspace: ~dev2/workspace  │
       │ Skills: atomic symlinks     │ │ Skills: atomic symlinks     │
       └──────────────┬──────────────┘ └──────────────┬──────────────┘
                      │                             │
                      └──────────────┬──────────────┘
                                     │ HTTP CONNECT (127.0.0.1:51128)
                                     ▼
                       ┌───────────────────────────┐
                       │    agy-proxy Service      │
                       │   - Loopback HTTP Proxy   │
                       │   - AES-256-GCM Vault     │
                       │   - Proactive Token Ref.  │
                       │   - 429 Intercept & Retry │
                       │   - 25s Debounce Guard    │
                       └─────────────┬─────────────┘
                                     │ HTTPS Outbound
                                     ▼
                         [ Google Antigravity API ]
```

---

## Key Features

1. **Strict Multi-User POSIX Isolation**:
   - Each developer operates under their dedicated Linux system account (POSIX UID/GID).
   - Home directories isolated with `0750` permissions.
   - Separate systemd user session units (`antigravity-web.service`) with background persistence via `loginctl enable-linger`.
   - Communication routed through individual unprivileged Unix domain sockets (`/run/user/<UID>/antigravity.sock`).

2. **Caddy Ingress & Automated TLS**:
   - Automated certificate management via Let's Encrypt / ZeroSSL.
   - Built-in HTTP to HTTPS redirection.
   - Transparent WebSocket buffering (`flush_interval -1`) for real-time terminal and editor synchronization.

3. **Central PAM Authentication & Session Portal (`agy-hub`)**:
   - Authenticates users against Linux PAM (`/etc/pam.d/antigravity-hub`).
   - Issues cryptographically signed HTTP-only, Secure `agy_session` JWT cookies.
   - Dynamically proxies incoming browser sessions to the correct developer's Unix domain socket.
   - Automatically manages IDE process lifecycle via systemd D-Bus.

4. **Quota Failover & Resilience Proxy (`agy-proxy`)**:
   - Transparent loopback HTTP proxy listening on `127.0.0.1:51128`.
   - Encrypted local credential vault (`/etc/antigravity/vault.enc`, AES-256-GCM).
   - Intercepts HTTP `429 Too Many Requests` / `ResourceExhausted` responses.
   - Applies a 25-second anti-flapping debounce guard, smoothly switching slots and replaying failed requests without interrupting developer IDE sessions.
   - Asynchronous background worker proactively refreshes Google OAuth tokens within 300s of expiration.

5. **Atomic Symlink Projection Engine**:
   - Central read-only skill catalog in `/opt/gemini/skills-library/` (16 presets, 101 skills).
   - Projects curated skills into user workspaces (`~<user>/workspace/.gemini/skills/`) via atomic directory replacement.
   - Enforces 5 permanent foundational skills (`ALWAYS_ACTIVE`): `agent-preset-conductor`, `grill-me`, `systematic-debugging`, `verification-before-completion`, `concise-planning`.

6. **Unified Management CLI (`agy`)**:
   - Full diagnostic suite: `agy doctor`
   - Preset switcher: `agy preset switch <preset-name>`
   - Vault management: `agy vault add <slot-id> <access-token> [refresh-token]`

---

## Quick Start / Installation

### Option 1: Automated One-Line Install (Recommended)

Run the universal installer on your Ubuntu, Debian, or RHEL-based server:

```bash
curl -fsSL https://raw.githubusercontent.com/nolekek/better-agy/main/install.sh | sudo bash
```

### Option 2: Pre-Packaged Release Tarball

Download the latest pre-packaged installer archive from [GitHub Releases](https://github.com/nolekek/better-agy/releases):

```bash
curl -fsSL -O https://github.com/nolekek/better-agy/releases/download/v0.1.0/better-agy-installer.tar.gz
tar -xzf better-agy-installer.tar.gz
cd better-agy-installer
sudo ./install.sh
```

### Option 3: Build & Install From Source

```bash
# 1. Clone repository
git clone https://github.com/nolekek/better-agy.git
cd better-agy

# 2. Run the installer
sudo ./install.sh
```

---

## Onboarding Developers

Add a new developer with automatic system user creation, home directory permissions, linger configuration, workspace directory initialization, and skill projection:

```bash
sudo add_developer <username> [preset] [password]
```

**Example:**
```bash
sudo add_developer alice web SecurePassword123!
```

Once provisioned:
1. Alice navigates to `https://your-domain.com`.
2. Logs in using her Linux credentials (`alice` / `SecurePassword123!`).
3. Her personal `antigravity-web.service` is activated on demand via D-Bus and proxied directly into her browser session.

---

## CLI Reference (`agy`)

### 1. Diagnostics & Health Check
Verify file permissions, systemd services, Caddy ingress, loopback proxy connectivity, PAM config, and vault integrity:

```bash
agy doctor
```

### 2. Preset Management
List available skill presets:
```bash
agy preset list
```

Switch active preset for a workspace:
```bash
agy preset switch backend --workspace /home/alice/workspace
```

### 3. Vault & Quota Management
Add an account slot with Google OAuth access and refresh tokens:
```bash
agy vault add slot_1 "ya29.a0AfH6SM..." "1//04..."
```

List configured slots and statuses:
```bash
agy vault list
```

---

## Repository Structure

```
better-agy/
├── crates/
│   ├── agy-cli/          # Unified management CLI (agy doctor, preset, vault)
│   ├── agy-core/         # Core business logic (symlink projection, presets)
│   ├── agy-hub/          # PAM portal, dynamic router, JWT sessions, web dashboard
│   ├── agy-proxy/        # Failover loopback proxy, AES-256-GCM vault, token refresh
│   └── agy-mock/         # Synthetic mock servers for offline testing
├── data/
│   └── skills-library/   # Central skill catalog (16 presets, 101 skills)
├── deploy/
│   ├── caddy/            # Caddyfile reverse proxy configuration
│   ├── scripts/          # install_server.sh, add_developer.sh
│   └── systemd/          # Service units (agy-hub, agy-proxy, antigravity-web)
├── tests/                # Comprehensive tier-1, tier-2, and e2e integration suites
├── install.sh            # Universal one-line installer
├── PROJECT.md            # In-depth architectural specification
└── TEST_INFRA.md         # Test methodology & acceptance criteria
```

---

## Running Tests

Run all unit, integration, and security test suites:

```bash
cargo test --workspace --exclude agy-tests
```

Run end-to-end integration test runner:
```bash
cargo test --test e2e_runner
```

---

## License

This project is licensed under the [MIT License](LICENSE).
