---
name: windows-desktop-control
description: Control the Windows desktop environment, open applications, inspect screen/UI elements, click, type, and manage windows via Windows-MCP.
---

# Windows Desktop Control & Automation

This skill guides the agent in controlling the Windows desktop and interacting with desktop applications.

## Available Capabilities via Windows-MCP:

1. **State Inspection (`State` / `Screenshot`):**
   - Retrieves the accessibility tree of the foreground window or entire desktop.
   - Provides exact coordinates $(x, y)$, element names, roles (Button, Edit, Window), and states.
   - For web browsers, `use_dom=True` filters out browser UI chrome and focuses on web content.

2. **Application Management (`App`):**
   - Launches apps by name (e.g. `notepad`, `calc`, `chrome`) or brings existing windows into focus.

3. **Input Emulation (`Click`, `Move`, `Type`, `Shortcut`, `Scroll`):**
   - `Click`: Clicks at specified $(x, y)$ coordinates.
   - `Type`: Types specified text into the focused field.
   - `Shortcut`: Triggers key combinations like `ctrl+c`, `ctrl+v`, `alt+tab`, `enter`.

4. **Clipboard & Processes:**
   - Read and write to Windows Clipboard.
   - List and manage running processes.

## Workflow Pattern:
1. `App` -> Launch or focus application.
2. `State` -> Read active screen elements and determine target coordinate $(x, y)$.
3. `Click` / `Type` -> Perform user interaction.
4. `State` -> Verify that the action resulted in the expected UI state.
