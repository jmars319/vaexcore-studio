# Release Readiness

This checklist is for local macOS release preparation. It does not replace notarization, but it keeps the build, sidecar, permissions, and signing state visible before a distributable build is cut.

## Checks

Run:

```bash
scripts/release-check.sh
```

The script runs:

- Rust formatting and tests.
- Sidecar build and preflight for the active Rust target triple.
- TypeScript typecheck and build.
- Tauri production build.
- bundle inspection for the `.app`, main executable, and bundled `media-runner`.
- `codesign` and `spctl` inspection when those tools are available.

## macOS Permissions

Current MVP preflight checks:

- Local API reachability.
- API token mode.
- Recording output folder writability.
- Managed `media-runner` health.
- Screen Recording permission through `CGPreflightScreenCaptureAccess` when display/window capture is enabled.
- Camera, microphone, and system audio as source-gated readiness entries.

Camera and microphone permission prompts should be wired when the real capture backend opens AVFoundation devices. System audio remains a future pipeline decision.

## Signing State

Local Tauri builds may be unsigned or ad hoc signed depending on the local toolchain and environment. For distribution:

- Use a Developer ID Application identity.
- Sign nested code, including `media-runner`.
- Enable hardened runtime.
- Provide the minimum required entitlements for actual capture features.
- Include user-facing usage descriptions for camera and microphone before enabling those sources in a real backend.
- Notarize the DMG and staple the ticket.

Useful inspection commands:

```bash
codesign -dvvv --entitlements :- "target/release/bundle/macos/vaexcore studio.app"
spctl -a -vv "target/release/bundle/macos/vaexcore studio.app"
plutil -p "target/release/bundle/macos/vaexcore studio.app/Contents/Info.plist"
```

## Release Blockers Before Real Media

- Native camera and microphone authorization checks.
- Source enumeration for displays, windows, cameras, and microphones.
- Real encoder/muxer capability detection.
- A concrete GStreamer, FFmpeg, or native macOS pipeline behind `media-runner`.
- Signed and notarized bundle with validated nested sidecar signing.
