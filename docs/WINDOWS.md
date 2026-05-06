# Windows Desktop

Studio is intended to be equal-capability on macOS and Windows. The Windows build uses the same Tauri app, the same local API, the same suite protocol, and Windows Credential Manager for stream-key storage.

## Windows 11 Build

Prerequisites:

- Node 20 or newer
- Rust stable with the MSVC toolchain
- Visual Studio Build Tools with Desktop development with C++
- WebView2 Runtime

Build the installer from this repo on Windows:

```sh
npm install
npm run app:build:windows
```

The build prepares the media sidecar with `scripts/prepare-sidecars.mjs`, builds the web UI, and packages the Tauri NSIS installer.
Set `TAURI_TARGET_TRIPLE` when preparing a sidecar for a non-host target. The
preflight check verifies the generated `apps/desktop/src-tauri/binaries/media-runner-*`
file before desktop crate tests or packaging run:

```sh
TAURI_TARGET_TRIPLE=x86_64-pc-windows-msvc npm run prepare:sidecars
TAURI_TARGET_TRIPLE=x86_64-pc-windows-msvc npm run check:sidecars
```

The suite-level build kit can also call:

```sh
npm run app:dist:windows
```

Studio also carries versioned Windows launchers under `tools/windows-launchers`.
The `.cmd` files can be double-clicked to start the full suite or an individual
installed app. `Install-VaexcoreLaunchers.cmd` creates Start Menu shortcuts and
a desktop suite shortcut using the suite logo.

## Local Paths

- Suite discovery: `%APPDATA%\vaexcore\suite`
- Studio app data: `%APPDATA%\com.vaexcore.studio` or the Tauri app data path
- Stream keys: Windows Credential Manager

macOS keeps using `~/Library/Application Support/vaexcore/suite` and macOS Keychain.
