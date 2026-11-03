---
name: lightcraft-photo
description: RAW photo development, non-destructive exposure/tone adjustment, color grading, HDR merging, and panorama stitching via LightCraft MCP.
---

# LightCraft: RAW Photo Developer & Color Grading

Use LightCraft for developing RAW camera files (Sony ARW, Nikon NEF, Canon CR3, Fujifilm RAF, Adobe DNG), applying color grading LUTs, merging exposure brackets into HDR, and stitching multi-shot panoramas.

## Engine & Protocol
- **MCP Server**: `artcraft-lightcraft` (`lightcraft-cli mcp`)
- **CLI Fallback**: `lightcraft-cli render`, `lightcraft-cli merge hdr|panorama`, `lightcraft-cli controls`
- **Supported Formats**: RAW (ARW, NEF, CR3, RAF), DNG, AVIF, JPEG XL (JXL), TIFF, Display P3 / Rec.2020

## Core Capabilities
1. **Parametric Tone & Exposure**:
   - Adjust exposure, highlight recovery, shadow lift, white balance, vibrance, and curves non-destructively.
2. **Computational Merge**:
   - Merge bracketed exposures into 32-bit HDR DNGs or stitch overlapping frames into wide-angle panoramas.
3. **Color Profile Calibration**:
   - Calibrate camera-specific color profiles for exact brand color consistency.

