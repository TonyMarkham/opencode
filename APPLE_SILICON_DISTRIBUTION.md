# Distributing OpenCode EGUI on Apple Silicon Macs

This document describes one concrete way to bundle the OpenCode EGUI client together with a local OpenCode server, STT model, and config for **Apple Silicon macOS (arm64)** so you can share it with other users as a `.app` (and optionally a `.dmg`).

The key points:

- The EGUI client now prefers a global `opencode` (from `PATH`) **but** falls back to an `opencode` binary located **next to the EGUI executable**.
- STT and models config are already wired to look for files **next to the executable**.
- That makes it easy to ship a self‑contained `.app` that "just works" on another Apple Silicon Mac.

---

## 1. Build the EGUI client (Apple Silicon)

From the repo root:

```bash
cd clients/egui
cargo build --release --target aarch64-apple-darwin
```

This produces a release binary at:

- `clients/egui/target/aarch64-apple-darwin/release/<egui-binary>`

For simplicity below, we’ll call this binary `opencode-egui`.

If your actual binary name differs, just substitute it anywhere you see `opencode-egui`.

---

## 2. Obtain the `opencode` server binary (arm64)

You need an **Apple Silicon** OpenCode server binary named `opencode`.

There are two typical options:

1. **From a release artifact** (recommended for non‑dev installs)
   - Download `opencode-darwin-arm64.zip` from your GitHub releases (same style used by the Zed extension).
   - Extract `opencode` from the archive.

2. **From your own build pipeline**
   - Use your existing Bun/packaging setup to produce an `opencode` executable for `darwin-arm64`.
   - Ensure the resulting file is executable (`chmod +x opencode` if needed).

We’ll assume you now have a standalone `opencode` binary that runs `opencode serve ...` on Apple Silicon.

---

## 3. Understand where the EGUI client looks for things

The EGUI code already assumes a few locations **relative to the EGUI executable**:

- **Whisper model** for STT
  - In `start_server_discovery`, if no explicit model path is configured, it auto‑detects:
    - `current_exe_dir/models/ggml-base.en.bin`
- **Models config**
  - `ModelsConfig::config_path()` resolves to:
    - `current_exe_dir/config/models.toml`
- **API keys (.env)**
  - `sync_api_keys_to_server` uses `dotenvy::dotenv()`, which looks for `.env` in the process CWD (or its parents).

With the recent spawn change, server startup now works like this when **no server is already running**:

1. Try `opencode` via `PATH` (current behavior).
2. If that fails with `ErrorKind::NotFound`, fall back to:
   - `current_exe_dir/opencode`

So if you bundle `opencode` **next to** the EGUI binary, the GUI will use that one automatically on machines where OpenCode is not installed globally.

---

## 4. Create a `.app` bundle layout

On macOS, an app is a folder with a specific structure and a `.app` extension. For example:

```text
OpenCode EGUI.app/
  Contents/
    Info.plist
    MacOS/
      opencode-egui      # the EGUI client binary
      opencode           # the bundled OpenCode server (arm64)
      models/
        ggml-base.en.bin
      config/
        models.toml
    Resources/
      .env.example
```

Notes:

- `MacOS/opencode-egui` and `MacOS/opencode` **must be executable**.
- `MacOS/models/ggml-base.en.bin` is where the STT auto‑detection looks by default.
- `MacOS/config/models.toml` is where `ModelsConfig` looks.
- `.env.example` in `Resources` is just a convenient place to ship a sample; you decide where you want the **real** `.env` file to live (see below).

### 4.1. Building the `.app` directory

From the repo root (one simple manual approach):

```bash
APP_ROOT="OpenCode EGUI.app"

# Create basic structure
mkdir -p "$APP_ROOT/Contents/MacOS"
mkdir -p "$APP_ROOT/Contents/Resources"

# Copy EGUI client (Apple Silicon build)
cp clients/egui/target/aarch64-apple-darwin/release/opencode-egui \
   "$APP_ROOT/Contents/MacOS/opencode-egui"

# Copy server binary (Apple Silicon)
cp path/to/your/opencode-darwin-arm64/opencode \
   "$APP_ROOT/Contents/MacOS/opencode"

# Copy STT model
cp path/to/ggml-base.en.bin \
   "$APP_ROOT/Contents/MacOS/models/ggml-base.en.bin"

# Copy models.toml
cp clients/egui/config/models.toml \
   "$APP_ROOT/Contents/MacOS/config/models.toml"

# Copy .env.example
cp clients/egui/.env.example \
   "$APP_ROOT/Contents/Resources/.env.example"
```

You also need an `Info.plist` at `OpenCode EGUI.app/Contents/Info.plist` with at least:

```xml
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
  <dict>
    <key>CFBundleName</key>
    <string>OpenCode EGUI</string>
    <key>CFBundleIdentifier</key>
    <string>com.example.opencode-egui</string>
    <key>CFBundleVersion</key>
    <string>1.0</string>
    <key>CFBundleShortVersionString</key>
    <string>1.0</string>
    <key>CFBundleExecutable</key>
    <string>opencode-egui</string>
  </dict>
</plist>
```

This tells macOS to launch `Contents/MacOS/opencode-egui` when the user opens the app.

> If you later introduce a launcher script (e.g. to force a specific CWD), set `CFBundleExecutable` to that script name instead.

---

## 5. Where should `.env` live?

The EGUI code calls `dotenvy::dotenv()`, which means:

- It looks for `.env` in the **current working directory** and its parents.

Two practical strategies when you ship a `.app`:

### Option A: `.env` next to the `.app`

- Ask users to:
  - Place `OpenCode EGUI.app` in a folder.
  - Put a `.env` file **next to** `OpenCode EGUI.app` (same folder).
- Then, ensure the app runs with its CWD set to that folder.

The easiest way to enforce that is a small launcher script in `Contents/MacOS` (and make that the `CFBundleExecutable`). For example:

```sh
#!/bin/sh
# Contents/MacOS/launcher

APP_DIR="$(cd "$(dirname "$0")/.." && pwd)"   # go up from Contents/MacOS to .app root
cd "$APP_DIR"                                   # CWD is the folder containing the .app
exec "${APP_DIR}/Contents/MacOS/opencode-egui"
```

Then in `Info.plist`:

```xml
<key>CFBundleExecutable</key>
<string>launcher</string>
```

With this setup:

- User puts `.env` next to `OpenCode EGUI.app`.
- `dotenvy::dotenv()` finds that `.env`.
- The EGUI client still finds:
  - `Contents/MacOS/opencode` (adjacent server, via the new spawn fallback).
  - `Contents/MacOS/models/ggml-base.en.bin`.
  - `Contents/MacOS/config/models.toml`.

### Option B: Application Support (more macOS‑native)

Alternatively, you can:

- Keep shipping `.env.example` in `Resources`.
- On first run, copy it to:
  - `~/Library/Application Support/opencode-egui/.env`
- Update `sync_api_keys_to_server` to call `dotenvy::from_filename(path)` with that explicit path instead of relying on CWD.

That’s slightly more code, but is closer to typical Mac conventions.

---

## 6. Creating a `.dmg` for sharing

Once `OpenCode EGUI.app` runs correctly on your machine, you can wrap it in a DMG for easier sharing.

Simple example from the repo root:

```bash
# Staging directory for the DMG contents
mkdir -p dist/OpenCode-EGUI-DMG
cp -R "OpenCode EGUI.app" dist/OpenCode-EGUI-DMG/

# Optionally also include .env.example alongside the app in the DMG root
cp clients/egui/.env.example dist/OpenCode-EGUI-DMG/.env.example

# Create compressed DMG
hdiutil create -volname "OpenCode EGUI" \
  -srcfolder dist/OpenCode-EGUI-DMG \
  -ov -format UDZO OpenCode-EGUI-arm64.dmg
```

You can now send `OpenCode-EGUI-arm64.dmg` to friends on Apple Silicon Macs:

1. They open the DMG.
2. Drag `OpenCode EGUI.app` into `/Applications`.
3. Place a `.env` next to the app (or in the location you chose, depending on Option A/B above).
4. Launch `OpenCode EGUI`.

On machines without a global OpenCode install:

- Discovery runs first and will **not** find an existing server.
- `spawn_and_wait()` starts:
  - `opencode` from `PATH` if present, otherwise
  - `Contents/MacOS/opencode` bundled with the app.
- STT and `models.toml` are loaded from `Contents/MacOS/models` and `Contents/MacOS/config`.

---

## 7. Code‑signing and notarization (optional but recommended)

To avoid scary macOS warnings when others run your app, you’ll eventually want to:

1. Obtain an Apple Developer ID certificate.
2. Sign the app:

   ```bash
   codesign --deep --force --options runtime \
     --sign "Developer ID Application: Your Name" \
     "OpenCode EGUI.app"
   ```

3. Notarize and staple (using `notarytool` and a keychain profile):

   ```bash
   xcrun notarytool submit "OpenCode EGUI.app" \
     --keychain-profile "your-profile" --wait

   xcrun stapler staple "OpenCode EGUI.app"
   ```

4. Recreate the DMG using the signed+notarized app.

You can still iterate on the basic bundling flow without signing while you’re testing with trusted friends.
