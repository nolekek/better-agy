---
name: effectcraft-vfx
description: Motion graphics, VFX compositing, timeline keyframing, layer masking, visual expressions, and JSX scripting via EffectCraft MCP.
---

# EffectCraft: Motion Graphics & VFX Compositing

Use EffectCraft for motion design, animated typography, particle effects, layer compositing, Roto Brush rotoscoping, camera movement, and After Effects-style JSX script execution.

## Engine & Protocol
- **MCP Server**: `artcraft-effectcraft` (`effectcraft-cli mcp`)
- **CLI Fallback**: `effectcraft-cli props`, `effectcraft-cli get/set`, `effectcraft-cli render`, `effectcraft-cli script`
- **Supported Formats**: `.ecproj`, `.aep`, JSX scripts, PNG sequence, ProRes, HEVC, WebM

## Animation & Compositing
1. **Comp & Layer Trees**:
   - Inspect composition hierarchy with `props <comp> <layer>`.
2. **Keyframing & Expressions**:
   - Manipulate spatial and temporal keyframes; apply mathematical expressions to layer properties.
3. **Motion Graphics Templates**:
   - Render parameterized motion templates (`.ectemplate`) dynamically with custom JSON parameters.

