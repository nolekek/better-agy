# Pre-Flight Product Launch Checklist

Before shipping any feature or product to users, complete this verification checklist to catch regressions, broken edge cases, and user friction.

---

## 1. Product & User Experience Readiness

- [ ] **Time-to-First-Value (TTFV)**: First-time user can reach the core value outcome without reading an external manual.
- [ ] **Empty States**: Every screen, view, or CLI command with 0 data displays a friendly message and a clear next step (CTA).
- [ ] **Loading Feedback**: Long-running operations (>1s) show a spinner, skeleton, or progress counter; UI remains responsive.
- [ ] **Human-Readable Error Messages**: Error dialogs/logs state what happened and provide a concrete recovery action. No cryptic stack traces displayed to end users.
- [ ] **Destructive Action Safety**: Irreversible actions (delete, overwrite, revoke) require explicit secondary confirmation.
- [ ] **Responsive / Cross-Platform**: Tested on target screen resolutions, operating systems, or terminal widths.

---

## 2. Technical & Performance Quality

- [ ] **Latency & P95**: Response times for critical paths meet agreed non-functional requirements (e.g. < 500ms for API, < 100ms for CLI responsiveness).
- [ ] **Graceful Degradation**: If an external API or secondary service is down, the core app functions in offline/read-only mode rather than hard crashing.
- [ ] **Timeouts & Retries**: All network requests have explicit timeout limits and retry with exponential backoff on transient errors.
- [ ] **Memory & Resource Leaks**: No runaway background tasks, unclosed connections, or infinite loops under load.

---

## 3. Security, Privacy & Compliance

- [ ] **No Leaked Credentials**: No hardcoded API keys, passwords, or secrets in code, git commits, or client-side bundles.
- [ ] **Input Sanitization**: User inputs are validated, sanitized, and escaped against injection attacks (SQLi, XSS, Command Injection).
- [ ] **Authorization Boundaries**: Users can only read and mutate their own data (IDOR checks passed).
- [ ] **Zero PII in Logs/Telemetry**: Audit logs and telemetry payloads are stripped of personal and sensitive data.

---

## 4. Telemetry & Observability

- [ ] **Core Events Verified**: Essential tracking events (`object_action`) fire accurately in test environments with expected metadata.
- [ ] **Error & Crash Tracking**: Unhandled exceptions are logged to an error tracker (e.g., Sentry, OpenTelemetry, or local error log).
- [ ] **Health Check / Heartbeat**: System has a mechanism or endpoint to verify operational status.

---

## 5. Documentation, Support & Rollback Plan

- [ ] **User Documentation**: Clear, concise README or release notes explaining what's new and how to get started.
- [ ] **Troubleshooting Section**: Common errors, known limitations, and FAQ documented.
- [ ] **Rollback Plan**: Clear step-by-step procedure to revert to the previous version in under 5 minutes if a critical bug emerges in production.
