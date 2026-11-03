---
name: filmcraft-video
description: Non-linear video editing (NLE), multi-track timeline sequencing, transitions, audio stem mixing, and media rendering via FilmCraft MCP.
---

# FilmCraft: Non-Linear Video Studio (NLE)

Use FilmCraft for assembling video timelines, trimming footage, synchronizing multi-track audio, creating transitions, title cards, and exporting broadcast-quality video files.

## Engine & Protocol
- **MCP Server**: `artcraft-filmcraft` (`filmcraft-cli mcp`)
- **CLI Fallback**: `filmcraft-cli exec <id>`, `filmcraft-cli inspect`, `filmcraft-cli export`, `filmcraft-cli render`
- **Supported Formats**: `.fcproj`, H.264, HEVC, Apple ProRes, Avid DNxHR, WAV, AAC, MP4, MOV

## Timeline Workflows
1. **Project & Sequence Setup**:
   - Inspect project assets and active sequence structure with `inspect sequence`.
2. **Precision Editing**:
   - Cut clips with `timeline.razor`, trim in/out points, slip/slide edits, and ripple deletions.
3. **Audio Stems & Levels**:
   - Balance dialog, music, and sound effects across dedicated audio tracks.
4. **Automated Export**:
   - Render draft or final sequences headlessly via `export <out> --preset <name>`.

