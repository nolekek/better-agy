# Project: Antigravity Linux Multi-User Server

## Architecture Overview
A production-grade, multi-user Linux server environment for Google Antigravity Web IDE (`antigravity-ide serve-web`) serving 2+ concurrent developers on a dedicated domain with automated TLS, strict POSIX user isolation, centralized PAM authentication, session-managed reverse proxying, resilient Google API quota failover via a dedicated proxy vault, and atomic symlink projection for skill presets.

```
                          [ Internet / Browser ]
                                     │
                                     ▼ HTTPS (Port 443 / 80)
                       ┌───────────────────────────┐
                       │   Caddy Ingress (R2)      │
                       │   - Automated ACME TLS    │
                       │   - WebSocket Upgrade     │
                       │   - flush_interval -1     │
                       └─────────────┬─────────────┘
                                     │ HTTP (127.0.0.1:8080)
                                     ▼
                       ┌───────────────────────────┐
                       │   agy-hub Service (R3)    │
                       │   - PAM Auth & JWT Cookie │
                       │   - Systemd D-Bus Control │
                       │   - Dynamic Proxy & WS    │
                       │   - Admin Dashboard SPA   │
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
                       │   agy-proxy Service (R4)  │
                       │   - Loopback HTTP Proxy   │
                       │   - AES-256-GCM Vault     │
                       │   - Proactive Token Ref.  │
                       │   - 429 Intercept & Retry │
                       │   - 25s Debounce Guard    │
                       └─────────────┬─────────────┘
                                     │ HTTPS Outbound
                                     ▼
                       Google Antigravity Endpoints
```

---

## Feature Inventory
| # | Feature | Description | Milestone | Source |
|---|---------|-------------|-----------|--------|
| 1 | Centralized Skills Library | `/opt/gemini/skills-library` containing 101 skills and 16 presets in Read-Only (`0755`) mode | M1 | R5 |
| 2 | Presets YAML Parser | Parsing `presets.yaml` defining 16 domain presets and 5 `ALWAYS_ACTIVE` meta-skills | M1 | R5 |
| 3 | Symlink Projection Engine | High-performance atomic creation/pruning of symlinks in `<workspace>/.gemini/skills/` without file renaming (< 5ms) | M1 | R5 |
| 4 | Immutable Guardrails | Mandatory automated inclusion of 5 ALWAYS_ACTIVE skills in every projected preset | M1 | R5 |
| 5 | Encrypted Account Vault | Local storage encrypted with AES-256-GCM storing multi-account OAuth credentials (`/etc/antigravity/vault.enc`, mode `0600`) | M2 | R4 |
| 6 | Proactive Token Refresh | Background asynchronous worker proactively refreshing Google OAuth tokens within 300s of expiration | M2 | R4 |
| 7 | Loopback HTTP Proxy | HTTP CONNECT proxy listening on `127.0.0.1:51128` filtering Google API calls vs raw TCP passthrough | M2 | R4 |
| 8 | In-Flight 429 Interception | Transparent interception of HTTP 429 and gRPC `ResourceExhausted` responses from upstream Google APIs | M2 | R4 |
| 9 | Debounce Protection | Strict 25-second debounce window preventing rapid slot thrashing during quota exhaustion bursts | M2 | R4 |
| 10 | Transparent Request Retry | Dynamic substitution of `Authorization: Bearer <token>` with next ready account slot and in-flight upstream retry | M2 | R4 |
| 11 | Linux POSIX Multi-User Isolation | Dedicated UID/GID per developer, group `agy-users`, home directory mode `0700`/`0750` | M3 | R1 |
| 12 | Systemd Linger & User Unit | `loginctl enable-linger` and `antigravity-web.service` managing per-user Web IDE lifetime | M3 | R1 |
| 13 | Linux PAM Authenticator | Non-blocking PAM verification (`pam_authenticate`) of Linux system accounts via worker thread | M3 | R3 |
| 14 | Secure JWT Session Management | `agy_session` cookie (HttpOnly, Secure, SameSite=Lax, 24h TTL) with user identity claims | M3 | R3 |
| 15 | Systemd D-Bus Lifecycle Controller | Asynchronous user unit control via zbus over `/run/user/<UID>/systemd/private` (`StartUnit`, `StopUnit`, status) | M3 | R3 |
| 16 | Web Management Dashboard | Embedded SPA dashboard for session view, real-time quota telemetry, and preset switching UI | M3 | R3 |
| 17 | Dynamic Unix Socket Reverse Proxy | HTTP reverse proxy forwarding requests from `/ide/*` to `/run/user/<UID>/antigravity.sock` with path rewriting | M4 | R3 |
| 18 | Raw Bidirectional WebSocket Pipeline | Zero-copy bidirectional streaming (`tokio::io::copy_bidirectional`) for terminal and LSP WebSocket connections | M4 | R3 |
| 19 | Caddy Ingress Gateway | Automated Let's Encrypt TLS, HTTP/2 & HTTP/3 termination, and reverse proxy to `127.0.0.1:8080` | M5 | R2 |
| 20 | Real-Time Token Stream Ingress | Caddy `flush_interval -1` preventing reverse proxy response buffering for streaming LLM tokens | M5 | R2 |
| 21 | Automated Server Provisioning | `install_server.sh` idempotent installation script setting up packages, users, permissions, and systemd units | M5 | R6 |
| 22 | Automated Developer Onboarding | `add_developer.sh` script creating users, configuring linger, workspace, and initial preset projection | M5 | R6 |
| 23 | Diagnostic Suite (`agy doctor`) | 9-category Rust diagnostic CLI tool validating kernel, PAM, ports, vault, sockets, linger, and egress | M5 | R6 |
| 24 | Synthetic Mock API Server | `agy-mock` server simulating HTTP 200 chunked streams, HTTP 429 quota exhaustion, and token refresh | E2E | AC |
| 25 | Full E2E Integration Suite | Tiers 1-4 comprehensive opaque-box test suite verifying end-to-end multi-user flow and failover | M6 | AC |
| 26 | Adversarial Hardening | Tier 5 white-box adversarial stress testing covering socket drops, quota bursts, and concurrent races | M6 | AC |

---

## Milestones
| # | Name | Scope | Dependencies | Status |
|---|------|-------|-------------|--------|
| M1 | `agy-core` & Symlink Projection Engine | Centralized skills library, `presets.yaml` parsing, 16 presets + 5 `ALWAYS_ACTIVE` meta-skills, atomic symlink engine | None | DONE |
| M2 | `agy-proxy` & Quota Failover Engine | AES-256-GCM Vault, proactive OAuth token refresh, loopback proxy (51128), 429/ResourceExhausted interception, 25s debounce, transparent retry | None | DONE |
| M3 | `agy-hub` Auth, Sessions & Systemd Lifecycle | PAM authentication, secure JWT session cookies, systemd user D-Bus controller, linger check, web management dashboard SPA | M1 | DONE |
| M4 | `agy-hub` Dynamic Reverse Proxy & WebSocket | Dynamic HTTP routing to per-user Unix domain sockets (`/run/user/<UID>/antigravity.sock`), zero-copy raw bidirectional WebSocket streaming | M3 | DONE |
| M5 | Ingress & Operations Tooling | Caddyfile (`flush_interval -1`, TLS), `install_server.sh`, `add_developer.sh`, `antigravity-web.service` template, `agy doctor` 9-category diagnostic CLI | M2, M4 | DONE |
| M6 | Final E2E Verification & Adversarial Hardening | Phase 1: 100% pass across E2E test suite (Tiers 1-4). Phase 2: Adversarial coverage hardening (Tier 5) | M1, M2, M3, M4, M5, TEST_READY | DONE |

*Note: E2E Testing Track runs in parallel with Implementation Track to build test infra, mock server, and test tiers 1-4.*

---

## Interface Contracts

### 1. `agy-core` ↔ `agy-hub` / `agy` CLI
- **Rust Trait / Functions:**
  ```rust
  pub struct PresetManifest {
      pub always_active: Vec<String>,
      pub presets: HashMap<String, Vec<String>>,
  }

  pub fn load_presets<P: AsRef<Path>>(path: P) -> Result<PresetManifest, CoreError>;

  pub fn sync_skills<P1: AsRef<Path>, P2: AsRef<Path>>(
      library_path: P1,
      target_skills_dir: P2,
      active_preset: &str,
      manifest: &PresetManifest,
  ) -> Result<SyncStats, CoreError>;
  ```
- **Guarantees:**
  - Files in `library_path` are opened Read-Only (`0755`). Never written or modified.
  - Symlinks in `target_skills_dir` are created atomically or pruned if obsolete.
  - Non-symlink user directories in `target_skills_dir` are strictly preserved.
  - Always includes the 5 `ALWAYS_ACTIVE` meta-skills in the resolved list.

### 2. `agy-proxy` Vault & Account Management
- **Vault File Structure (`/etc/antigravity/vault.enc`):**
  ```rust
  pub struct AccountSlot {
      pub id: String,
      pub email: String,
      pub access_token: String,
      pub refresh_token: String,
      pub expires_at: i64,      // Unix timestamp (seconds)
      pub state: SlotState,     // Ready, Active, Cooldown(until), Disabled
  }

  pub struct EncryptedVaultPayload {
      pub nonce: [u8; 12],
      pub ciphertext: Vec<u8>, // AES-256-GCM encrypted JSON array of AccountSlot
  }
  ```
- **Failover Contract:**
  - On HTTP 429 / `ResourceExhausted`:
    - Check debounce: if `now - last_failover < 25s`, reject rapid switch.
    - Set current slot to `Cooldown(now + 300s)`.
    - Select next slot where `state == Ready` and `expires_at > now + 60s`.
    - Update in-memory request `Authorization: Bearer <new_token>`.
    - Replay request upstream. Return 200 stream to client.

### 3. `agy-hub` Dynamic Router ↔ Systemd User Sockets
- **Socket Path:** `/run/user/<UID>/antigravity.sock`
- **HTTP Routing:**
  - Client request: `GET /ide/<username>/<path...>` with valid `agy_session` JWT cookie.
  - Verify `jwt.uid == target_uid` (or admin role).
  - Check socket readiness: if missing, trigger `StartUnit("antigravity-web.service")` via D-Bus and poll with 10s timeout.
  - Connect to Unix domain socket via `tokio::net::UnixStream`.
  - Rewrite request URI path from `/ide/<username>/<path...>` to `/<path...>`.
  - Pipe response chunks back to client with HTTP chunked transfer encoding.
- **WebSocket Upgrade:**
  - Detect `Upgrade: websocket` and `Connection: Upgrade`.
  - Perform `hyper::upgrade::on(req)`.
  - Connect to Unix domain socket.
  - Execute `tokio::io::copy_bidirectional(&mut client_io, &mut upstream_io)`.

---

## Code Layout
```
d:\better-agy/
├── Cargo.toml                       # Cargo workspace manifest
├── Cargo.lock
├── PROJECT.md                       # Master architecture & milestones specification
├── TEST_INFRA.md                    # E2E test track specification
├── crates/
│   ├── agy-core/                    # Shared library: presets, symlinks, errors
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── preset.rs            # YAML parser & ALWAYS_ACTIVE validation
│   │       ├── symlink.rs           # Atomic symlink engine
│   │       └── error.rs
│   ├── agy-proxy/                   # Transparent loopback proxy & vault
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── main.rs
│   │       ├── vault.rs             # AES-256-GCM storage
│   │       ├── proxy.rs             # HTTP CONNECT & MITM interceptor
│   │       ├── failover.rs          # 429 detection & 25s debounce
│   │       ├── refresh.rs           # Background token refresher
│   │       └── state.rs
│   ├── agy-hub/                     # Central gateway & reverse proxy
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── main.rs
│   │       ├── auth/                # PAM & JWT sessions
│   │       │   ├── mod.rs
│   │       │   ├── pam.rs
│   │       │   └── jwt.rs
│   │       ├── proxy/               # Dynamic router & WS upgrade
│   │       │   ├── mod.rs
│   │       │   ├── router.rs
│   │       │   └── websocket.rs
│   │       ├── systemd/             # D-Bus zbus controller
│   │       │   └── manager.rs
│   │       ├── dashboard/           # Web UI SPA endpoints
│   │       │   ├── mod.rs
│   │       │   └── static_files.rs
│   │       └── web/                 # HTML/CSS/JS frontend assets
│   ├── agy-cli/                     # CLI operations & doctor diagnostic tool
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── main.rs
│   │       ├── doctor/              # 9-category diagnostic suite
│   │       └── commands/
│   └── agy-mock/                    # Synthetic Antigravity API server for testing
│       ├── Cargo.toml
│       └── src/
│           ├── lib.rs
│           └── server.rs
├── deploy/                          # Infrastructure & provisioning scripts
│   ├── caddy/
│   │   └── Caddyfile                # Ingress configuration
│   ├── systemd/
│   │   ├── agy-proxy.service        # System unit
│   │   ├── agy-hub.service          # System unit
│   │   └── antigravity-web.service  # User unit template
│   └── scripts/
│       ├── install_server.sh        # Server installer
│       └── add_developer.sh         # User on-boarding script
├── data/
│   └── skills-library/              # Central library source (101 skills, 16 presets)
│       ├── presets.yaml
│       └── <101 skill folders>/
└── tests/                           # E2E test suite (Tiers 1-4, Tier 5)
    ├── e2e_runner.rs
    ├── tier1_features/
    ├── tier2_boundaries/
    ├── tier3_pairwise/
    ├── tier4_scenarios/
    └── tier5_adversarial/
```
