---
name: figcraft-design
description: UI/UX design systems, component tokens, responsive layouts, Figma/Penpot file inspection, and frame-to-code mapping via FigCraft MCP.
---

# FigCraft: UI/UX Design System Studio

Use FigCraft for building and inspecting user interface designs, extracting design tokens (colors, typography, spacing, shadows), reading Penpot/Figma files, and mapping components to code.

## Engine & Protocol
- **MCP Server**: `artcraft-figcraft` (`figcraft-cli mcp`)
- **CLI Fallback**: `figcraft-cli info`, `figcraft-cli export`, `figcraft-cli render`, `figcraft-cli connect`
- **Supported Formats**: `.figcraft`, `.penpot`, `.fig`, `.sketch`, `.json`, `.svg`, `.png`

## Bridge to Frontend
1. **Design Token Extraction**:
   - Read variables, color palettes, typographic scales, and grid definitions into Tailwind config or CSS tokens.
2. **Component Mapping (Code Connect)**:
   - Map FigCraft frames directly to Next.js / React components for pixel-perfect frontends.
3. **Asset & Frame Export**:
   - Export individual frames or icons into SVG/WebP headlessly during build steps.

