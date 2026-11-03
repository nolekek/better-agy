---
name: cadcraft-drafting
description: 2D and 3D CAD drafting, DXF/DWG conversion, architectural floor plans, engineering schematics, coordinate geometry and dimensioning via CADCraft MCP.
---

# CADCraft Drafting & Technical Schematics

Use CADCraft when working with architectural plans, 2D/3D mechanical drawings, DXF/DWG files, electrical schematics, coordinate layouts, or dimensioned technical drawings.

## Engine & Protocol
- **MCP Server**: `artcraft-cadcraft` (`cadcraft-cli mcp`)
- **CLI Fallback**: `cadcraft-cli info <drawing>`, `cadcraft-cli convert <IN> <OUT>`, `cadcraft-cli run`
- **Supported Formats**: `.dwg`, `.dxf`, `.dwt`, `.svg`, `.pdf`, `.png`

## Core Workflows
1. **Inspect Drawing**:
   - Call `cadcraft-cli info <file>` or use MCP drawing inspection tools to retrieve layer tables, block definitions, linetypes, limits, and object counts.
2. **Format Conversion**:
   - High-fidelity conversion between AutoCAD formats (`.dwg` ↔ `.dxf`) or vector presentation (`.svg`, `.pdf`).
3. **Parametric Scripting**:
   - Use `cadcraft-cli run` with command scripts to automate geometry generation: lines, circles, polylines, hatches, dimensions, text annotations.
4. **Integration with Hardware Preset**:
   - Pair with `solidcraft-modeling` for sheet metal unfolding, 2D profiling, and CNC router G-code generation.

