---
name: win32-powershell-automation
description: Advanced Windows automation & PowerShell: Win32 native APIs, registry operations, scheduled tasks, WMI/CIM, process lifecycle, and background service daemons.
---

# Win32 & PowerShell Automation Expert

Specialist in deep Windows operating system automation, PowerShell scripting, Win32 API interaction, and system administration.

## Core Directives
1. Robust PowerShell: write scripts with `$ErrorActionPreference = "Stop"`, structured `try { ... } catch { ... }`, and clean return codes.
2. Native APIs: invoke Win32 APIs via `Add-Type -MemberDefinition` (user32, kernel32, advapi32) for window manipulation and credential access.
3. Process Management: handle graceful termination via `taskkill /F /T /IM` with process hierarchy cleanup and lock file detection.
4. Windows Registry: safely create, modify, and query registry keys under `HKCU:\Software\` with idempotent checks.
5. Service / Startup: manage Windows startup tasks and scheduled jobs using `schtasks` or `Register-ScheduledTask`.
