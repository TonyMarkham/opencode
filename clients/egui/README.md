# OpenCode EGUI Client

Native Rust desktop client for OpenCode using EGUI.

## Quick Start

### Prerequisites
1. Install cargo-make:
   ```bash
   cargo install cargo-make
   ```

2. Download Whisper model for speech-to-text:
   ```bash
   curl -L https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-base.en.bin \
        -o ~/Downloads/ggml-base.en.bin
   ```

### Running

```bash
cargo make dev
```

This will:
- Build the project
- Copy the Whisper model to `target/debug/models/`
- Run the application

### Push-to-Talk

Once running with the model configured:
- **Press and hold `AltRight`** to record
- **Release `AltRight`** to stop and transcribe
- Transcribed text appears in the input field

## Manual Setup

If you don't want to use cargo-make:

1. Build: `cargo build`
2. Place your Whisper model at `target/debug/models/ggml-base.en.bin`
3. Run: `cargo run`

Alternatively, configure a custom model path in Settings > Audio.

## Features

- Auto server discovery and spawning
- Multi-session tabs
- Real-time streaming with markdown rendering
- Tool call visualization
- Speech-to-text (push-to-talk with AltRight)
- Configurable UI (fonts, chat density)

## Architecture

See [EGUI_PLAN.md](./EGUI_PLAN.md) and [STT_PLAN.md](./STT_PLAN.md) for detailed architecture and implementation plans.
