# VaexCore Icon System

VaexCore uses responsive app icons.

- The v3 Studio brand artwork is the source of truth for the in-app logo and icon generation.
- Large brand art includes the detailed neon scene, Studio wordmark, and `STUDIO` label.
- Small app icon slots use a center crop from the same v3 art so the `V` mark and streaming/recording panel stay recognizable without tiny text.

For macOS, `apps/desktop/src-tauri/icons/icon.icns` uses the v3 center crop through 256 px and the full v3 Studio brand artwork for 512 px and 1024 px slots.
