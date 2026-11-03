---
name: texturecraft-pbr
description: 3D PBR texturing, material mapping, UV island validation, normal/roughness/metallic maps, and surface finishing via TextureCraft MCP.
---

# TextureCraft: 3D PBR Material Studio

Use TextureCraft for physical based rendering (PBR) texturing, surface finishes, UV coordinate inspection, and generating texture sets for 3D game engines and product rendering.

## Engine & Protocol
- **MCP Server**: `artcraft-texturecraft` (`texturecraft-cli mcp`)
- **CLI Fallback**: `texturecraft-cli info <project.tcraft|mesh>`, `texturecraft-cli commands`
- **Supported Formats**: `.tcraft`, `.obj`, `.gltf`, Albedo, Roughness, Metallic, Normal, Ambient Occlusion maps

## Workflows
1. **Mesh & UV Inspection**:
   - Inspect triangle counts, UV distortion, overlapping islands, and material boundaries before painting.
2. **PBR Channel Management**:
   - Build layered materials with non-destructive masks, curvature maps, and ambient occlusion baking.
3. **Product Visualization**:
   - Connect textured assets directly into `lightcraft-photo` or web 3D visualizers.

