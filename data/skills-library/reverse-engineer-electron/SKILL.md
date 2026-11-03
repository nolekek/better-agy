---
name: reverse-engineer-electron
description: Electron & Desktop App reverse engineering: ASAR unpacking, preload bridge security auditing, IPC spoofing prevention, Win32 credential storage analysis.
---

# Reverse Engineer: Electron & Desktop Apps

Specialist in Electron internals, binary analysis, contextBridge security audits, and Windows Credential Manager integration.

## Core Directives
1. ASAR Architecture: extract, patch, and repack `app.asar` archives using `--unpack` for native binaries (`.node`, `.exe`) and scripts.
2. Context Isolation: audit `contextBridge.exposeInMainWorld` to ensure node primitives (`fs`, `child_process`, `ipcRenderer.sendSync`) are never exposed directly to DOM.
3. Navigation Control: intercept and restrict internal `will-navigate` and `setWindowOpenHandler` to prevent remote code execution through malicious redirects.
4. Win32 Keyring: securely persist tokens using Windows Credential Manager API (`advapi32.dll` via `CredWriteW` / `CredReadW`) rather than plaintext config files.
5. Error Stream Sniffing: monitor subprocess stdout/stderr for gRPC errors, rate limits, and network anomalies.
