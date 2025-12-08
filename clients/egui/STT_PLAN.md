# Speech-to-Text (STT) Integration for OpenCode EGUI

Status: **COMPLETE** (2025-12-08)  
Reference: bmad-engine audio/stt implementation

Phases 1-4 fully implemented. Phase 5 (Settings UI) deferred.

## Goals
- Add push-to-talk voice input using local Whisper model
- Transcribe user speech and insert into message input field
- Provide visual feedback during recording and transcription
- Support configurable push-to-talk key binding

## Non-Goals
- Text-to-speech (TTS) output (defer to future)
- Real-time streaming transcription (use offline batch mode)
- Cloud-based STT services (use local Whisper only)

## Architecture Overview

### Component Structure
Reuse bmad-engine's audio/stt modules with minimal changes:

```
src/
  audio/
    mod.rs          - AudioManager coordinator
    error.rs        - AudioError enum (thiserror-based)
    stt/
      mod.rs        - re-exports
      capture.rs    - AudioCapturer (cpal-based mic capture)
      engine.rs     - SttEngine (whisper-rs wrapper)
      resampler.rs  - Resampler (rubato, device rate → 16kHz)
```

### Data Flow
1. User presses push-to-talk key (default: AltRight)
2. UI detects via `raw_input_hook`, sends StartRecording to async task
3. AudioManager starts cpal stream, buffers samples
4. User releases key
5. UI sends StopRecording, async task:
   - Stops capture, gets raw samples
   - Resamples to 16kHz
   - Passes to Whisper for transcription (blocking)
6. Transcription result sent back to UI via channel
7. UI appends text to active tab's input field

### State Machine
```
RecordingState:
  Idle → (key_down) → Recording
  Recording → (key_up) → Idle
```

Ignore key repeats and other state transitions to prevent double-triggers.

## Integration Points

### 1. UI Event Handling
Add `raw_input_hook` to `OpenCodeApp` (same pattern as bmad-engine):
- Monitor configured push-to-talk key
- Track `recording_state: RecordingState`
- Send messages to audio task via existing `ui_tx` channel

### 2. Async Audio Task
Spawn persistent tokio task on app startup:
- Receives: `StartRecording`, `StopRecording` messages
- Owns: `AudioManager` instance
- Sends back: `RecordingStarted`, `RecordingStopped`, `Transcription(String)`, `AudioError`
- Runs transcription on tokio blocking pool (Whisper is CPU-bound)

### 3. UiMsg Extensions
Add variants to existing `UiMsg` enum:
```rust
enum UiMsg {
    // ... existing variants
    RecordingStarted,
    RecordingStopped,
    Transcription(String),
    AudioError(String),
}
```

### 4. Visual Feedback
- Recording: Show microphone icon in input panel or status bar
- Processing: Show spinner after key release
- Success: Insert text, show brief success indicator
- Error: Show error message (e.g., "No microphone found")

## Dependencies
Add to Cargo.toml:
```toml
cpal = "0.15"        # Audio capture
hound = "3.5"        # WAV encoding (optional, for debugging)
rubato = "0.15"     # Sample rate conversion
whisper-rs = "0.15" # Local Whisper STT
```

Note: whisper-rs requires the Whisper model file (ggml format). User must provide path via config.

## Configuration
Extend `config.rs`:
```rust
#[derive(Serialize, Deserialize, Clone)]
pub struct AudioConfig {
    pub push_to_talk_key: String,  // e.g., "AltRight"
    pub whisper_model_path: Option<String>,
}
```

Default: AltRight key, no model (STT disabled until user configures)

## Model Setup
User must download a Whisper model (e.g., `ggml-base.en.bin`):
- Small models: ~150MB, fast, English-only
- Larger models: 1-3GB, more accurate, multilingual
- Recommend: base.en for speed/accuracy balance

Settings UI should:
- Show model path input
- Validate model file exists
- Show error if STT used without valid model

## Error Handling
Graceful degradation:
- No microphone → disable push-to-talk, show warning
- No model configured → ignore push-to-talk, show instructions
- Transcription fails → show error, keep input field unchanged
- Audio capture fails → reset state, show error

## Key Architectural Questions

### Q1: Audio task lifecycle
**Options:**
- A) Spawn audio task on app startup, keep alive
- B) Spawn audio task lazily on first recording
- C) Spawn/destroy audio task per recording

**Decision:** Option A. AudioManager initialization (loading Whisper model) is expensive (~1s). Do it once at startup if model is configured.

### Q2: Transcription blocking
**Options:**
- A) Run Whisper on tokio blocking pool
- B) Use rayon thread pool
- C) Spawn std::thread per transcription

**Decision:** Option A. Reuse existing tokio runtime, use `spawn_blocking`. Whisper transcription is CPU-bound and takes 0.5-2s depending on audio length.

### Q3: Error recovery
**Options:**
- A) Restart AudioManager on any error
- B) Keep AudioManager alive, reset state only
- C) Disable STT entirely until app restart

**Decision:** Option B. Most errors (empty audio, no mic) are transient. Only fatal errors (model load failure) should disable STT.

### Q4: Model validation
**Options:**
- A) Validate model on config save
- B) Validate model when audio task starts
- C) Validate model on first recording attempt

**Decision:** Option B. Check file exists when spawning audio task. This provides immediate feedback without blocking UI during config changes.

## Implementation Plan

### Phase 1: Core Audio Infrastructure ✓ COMPLETE
1. ✓ Copied audio/ modules from bmad-engine
2. ✓ Added dependencies to Cargo.toml (cpal, rubato, whisper-rs)
3. ✓ Added AudioConfig to config.rs
4. ✓ Verified compilation

### Phase 2: Async Audio Task ✓ COMPLETE
1. ✓ Created audio task that owns AudioManager
2. ✓ Handles StartRecording/StopRecording messages
3. ✓ Sends results back via UI channel (RecordingStarted, RecordingStopped, Transcription, AudioError)
4. ✓ Added error handling and graceful degradation
5. ✓ Spawns task on app startup if Whisper model is configured
6. ✓ Transcription runs in dedicated audio task (non-blocking for UI)

### Phase 3: UI Integration ✓ COMPLETE
1. ✓ Added RecordingState enum (Idle, Recording)
2. ✓ Implemented raw_input_hook for push-to-talk detection
3. ✓ State machine: (Idle, key_down) → Recording, (Recording, key_up) → Idle
4. ✓ Sends AudioCmd messages to audio task on key press/release
5. ✓ Handles audio messages in poll_ui_msgs (RecordingStarted, RecordingStopped, Transcription, AudioError)
6. ✓ Inserts transcription into active tab's input field with space separator
7. ✓ Resets state on errors

### Phase 4: Visual Feedback ✓ COMPLETE
1. ✓ Audio status messages render in chat history as system messages (role="system")
2. ✓ Recording: 🎙 "Recording..." appears when key pressed
3. ✓ Processing: "Processing audio..." appears when key released
4. ✓ Success: ✅ "Transcription complete" appears when done
5. ✓ Error: ⚠ "Audio: <error>" appears on failure
6. ✓ Messages don't interfere with input panel layout (no jumping)
7. ✓ Colored emoji rendering via egui-twemoji:
   - Added egui-twemoji dependency (forked for egui 0.33 compatibility)
   - System messages use EmojiLabel for colored emoji rendering
   - Removed NotoEmoji font (was interfering with twemoji image rendering)
   - Emojis without variation selectors for better twemoji compatibility
8. ✓ Tool call status icons also use colored emojis (✅ success, ❌ error, ⏳ in-progress)

### Phase 5: Settings UI
1. Add audio section to Settings window
2. Model path input with file picker
3. Push-to-talk key configuration
4. Test STT enablement flow

## Testing Strategy
- Manual: Press AltRight, speak, verify transcription appears
- Error cases: No mic, no model, empty audio
- Long audio: 10+ seconds, verify non-blocking
- Multiple tabs: Verify transcription goes to active tab only

## Risks & Mitigations
- **Whisper model size:** Models are 150MB-3GB. Mitigate: Recommend base.en (145MB), provide download instructions.
- **Transcription latency:** 0.5-2s depending on audio length. Mitigate: Show "Processing..." indicator, use faster model.
- **CPAL platform issues:** Some audio devices/drivers buggy. Mitigate: Graceful error messages, allow disable.
- **Model not found:** User forgets to download. Mitigate: Clear setup instructions in Settings UI.

## Future Enhancements (Out of Scope)
- Auto-send after transcription (optional flag)
- Voice activity detection (auto-stop when silence)
- Multiple language support
- Custom Whisper parameters (temperature, etc.)
- TTS output for assistant responses
- Hotkey customization UI (key picker widget)
