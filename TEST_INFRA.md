# E2E Test Infra: Antigravity Linux Multi-User Server

## Test Philosophy
- **Requirement-Driven & Opaque-Box**: Tests validate externally observable behaviors (HTTP responses, JWT authentication, socket connectivity, process lifecycle, symlink creation, 429 quota failover) without relying on internal function implementations.
- **Methodology**: 4-tier systematic approach (Category-Partition, Boundary Value Analysis, Pairwise Combinatorial, and Real-World Workloads) + Tier 5 Adversarial Coverage Hardening.
- **Progressive Testability**: Verification mechanisms do not require features more complex than what they verify. Early tiers (Tier 1) provide immediate pass/fail signals on isolated modules.

---

## Feature Inventory Mapping
| # | Feature | Requirements Source | Tier 1 | Tier 2 | Tier 3 | Tier 4 |
|---|---------|---------------------|:------:|:------:|:------:|:------:|
| 1 | Centralized Skills Library | R5 (Library structure) | ≥5 | ≥5 | ✓ | ✓ |
| 2 | Presets YAML Parsing | R5 (16 presets + 5 meta) | ≥5 | ≥5 | ✓ | ✓ |
| 3 | Symlink Projection Engine | R5 (Atomic creation & pruning) | ≥5 | ≥5 | ✓ | ✓ |
| 4 | ALWAYS_ACTIVE Guardrails | R5 (Mandatory meta-skills) | ≥5 | ≥5 | ✓ | ✓ |
| 5 | Encrypted Account Vault | R4 (AES-256-GCM storage) | ≥5 | ≥5 | ✓ | ✓ |
| 6 | Proactive Token Refresh | R4 (OAuth worker) | ≥5 | ≥5 | ✓ | ✓ |
| 7 | Loopback HTTP Proxy | R4 (HTTP CONNECT 51128) | ≥5 | ≥5 | ✓ | ✓ |
| 8 | In-Flight 429 Interception | R4 (Interception) | ≥5 | ≥5 | ✓ | ✓ |
| 9 | 25s Debounce Protection | R4 (Debounce boundary) | ≥5 | ≥5 | ✓ | ✓ |
| 10 | Transparent Request Retry | R4 (Token swap & retry) | ≥5 | ≥5 | ✓ | ✓ |
| 11 | POSIX User Isolation | R1 (UID/GID & 0700/0750) | ≥5 | ≥5 | ✓ | ✓ |
| 12 | Systemd Linger & User Service | R1 (`loginctl`, user unit) | ≥5 | ≥5 | ✓ | ✓ |
| 13 | PAM Authentication | R3 (Linux credentials) | ≥5 | ≥5 | ✓ | ✓ |
| 14 | JWT Session Cookies | R3 (HttpOnly, Secure, TTL) | ≥5 | ≥5 | ✓ | ✓ |
| 15 | Systemd D-Bus Lifecycle | R3 (zbus private socket) | ≥5 | ≥5 | ✓ | ✓ |
| 16 | Web Management Dashboard | R3 (UI, quota & presets) | ≥5 | ≥5 | ✓ | ✓ |
| 17 | Dynamic Socket Reverse Proxy | R3 (HTTP -> user socket) | ≥5 | ≥5 | ✓ | ✓ |
| 18 | Raw Bidirectional WebSocket | R3 (Zero-copy WS upgrade) | ≥5 | ≥5 | ✓ | ✓ |
| 19 | Caddy Ingress Gateway | R2 (TLS, WS, reverse proxy) | ≥5 | ≥5 | ✓ | ✓ |
| 20 | Real-Time Token Streaming | R2 (`flush_interval -1`) | ≥5 | ≥5 | ✓ | ✓ |
| 21 | Automated Server Installer | R6 (`install_server.sh`) | ≥5 | ≥5 | ✓ | ✓ |
| 22 | Developer Onboarding | R6 (`add_developer.sh`) | ≥5 | ≥5 | ✓ | ✓ |
| 23 | Diagnostic Suite (`agy doctor`) | R6 (9 categories, JSON) | ≥5 | ≥5 | ✓ | ✓ |
| 24 | Synthetic Mock API Server | AC (200, 429, streaming) | ≥5 | ≥5 | ✓ | ✓ |

---

## Test Architecture & Mock Strategy
- **Synthetic Mock Server (`agy-mock`)**:
  - Simulates Google Antigravity endpoints (`/v1internal:generateContent`, `/oauth2.googleapis.com/token`).
  - Capable of programmed response sequencing (e.g. Return 429 once, then 200 on retry).
  - Emulates chunked streaming of LLM tokens to verify no buffering occurs across Caddy and `agy-hub`.
- **Test Runner**:
  - `cargo test --workspace` for hermetic unit and integration tests.
  - Dedicated E2E runner in `tests/e2e_runner.rs` executing integration scenarios with mock endpoints and synthetic user environments.
- **Pass/Fail Semantics**:
  - Exit code 0 on all tests.
  - 100% PASS requirement across all tiers.

---

## Real-World Application Scenarios (Tier 4)
| # | Scenario | Features Exercised | Complexity |
|---|----------|--------------------|------------|
| 1 | Multi-Tenant Onboarding & IDE Access | F11, F12, F13, F14, F15, F17, F22 | High |
| 2 | High-Concurrency LLM Streaming via Ingress | F17, F18, F19, F20, F24 | High |
| 3 | In-Flight Quota Exhaustion & Debounced Failover | F5, F7, F8, F9, F10, F24 | Very High |
| 4 | Rapid Preset Hot-Swapping across Concurrent Users | F1, F2, F3, F4, F16 | Medium |
| 5 | Full Server Cold-Boot & Self-Healing Audit | F12, F15, F21, F23 | High |

---

## Coverage Minimum Thresholds
- Given 24 core features:
  - Tier 1: ≥ 120 test cases (≥5 per feature, happy path)
  - Tier 2: ≥ 120 test cases (boundary & corner cases, e.g. 24,999ms vs 25,001ms debounce, empty vault, invalid PAM, malformed YAML)
  - Tier 3: ≥ 24 pairwise cross-feature interaction cases
  - Tier 4: ≥ 5 realistic end-to-end workload scenarios
- Total Target Minimum: ~269 test cases
