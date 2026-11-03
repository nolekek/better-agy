---
name: solidcraft-modeling
description: Parametric 3D mechanical CAD modeling, feature history tree, constraint solving, assemblies, STEP/STL export, and stress analysis via SolidCraft MCP.
---

# SolidCraft: Parametric 3D Solid CAD Modeling

Use SolidCraft for mechanical engineering, physical device enclosures, 3D printable components, parametric assemblies, STEP file analysis, and constraint-based solid modeling.

## Engine & Protocol
- **MCP Server**: `artcraft-solidcraft` (`solidcraft-cli mcp --headless --allow-any-path`)
- **CLI Fallback**: `solidcraft-cli run`, `solidcraft-cli snapshot`, `solidcraft-cli eval`, `solidcraft-cli render`
- **Supported Formats**: `.sc` (native), `.step` / `.stp` (ISO 10303 exchange), `.stl` (mesh), `.obj`

## Modeling Guidelines
1. **Parametric Feature Tree**:
   - Work methodically: Sketch $	o$ Extrude/Revolve $	o$ Fillet/Chamfer $	o$ Boolean Cut/Union $	o$ Shell/Hole.
   - Every feature is non-destructive and re-evaluates via the geometric constraint solver.
2. **Headless Execution & Snapshots**:
   - Use `solidcraft-cli snapshot --in model.sc --out preview.png` to verify geometry visually from camera perspectives (`iso`, `top`, `front`).
3. **Assembly & Tolerances**:
   - Inspect clearance, interference, and mating constraints between parts before exporting to production.
4. **Exchange Formats**:
   - Always verify STEP export integrity with `solidcraft-cli eval` for manufacturing readiness.

