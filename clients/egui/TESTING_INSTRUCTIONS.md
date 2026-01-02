# Testing Instructions for macOS Sleep/Wake Fix

## Summary of Changes

The segmentation fault on macOS sleep/wake has been fixed by updating the `change_gl_context` function in `/Users/tony/git/egui/crates/eframe/src/native/glow_integration.rs`.

### What was changed:
- Replaced `.unwrap()` calls with proper error handling in the `change_gl_context` function
- When `make_not_current()` or `make_current()` fails (due to invalid context after sleep), the error is logged and the frame is skipped
- The application continues running instead of crashing

## Testing Steps

1. **Build the application:**
   ```bash
   cd /Users/tony/git/opencode/clients/egui
   cargo build --release
   ```

2. **Run the application:**
   ```bash
   ./target/release/opencode-egui
   ```

3. **Test sleep/wake cycle:**
   - Let the Mac go to sleep (you can force it via **Apple menu → Sleep**)
   - OR wait for automatic sleep if configured
   - Wake the Mac (mouse movement or keyboard press)

4. **Expected behavior:**
   - ✅ The app should **NOT** crash with "segmentation fault"
   - ✅ You may see log messages in the console: `make_current failed (likely due to invalid context after sleep/wake): <error>`
   - ✅ The app may skip 1-2 frames during wake
   - ✅ Rendering should resume normally after the context is restored

5. **What to look for:**
   - If the app crashes, check the console output for error messages
   - If you see `make_current failed` or `make_not_current failed` in logs, that's expected during sleep/wake
   - The app should remain responsive after wake

## Debugging

If you want to see detailed logs:

```bash
RUST_LOG=debug,eframe=trace ./target/release/opencode-egui
```

This will show all context switching operations and errors.

## Current Configuration

The fix is in the local egui fork at `/Users/tony/git/egui/`. 

Your `Cargo.toml` is configured to use local path dependencies:
- `eframe` → `/Users/tony/git/egui/crates/eframe`
- `egui` → `/Users/tony/git/egui/crates/egui`
- `egui_extras` → `/Users/tony/git/egui/crates/egui_extras`

## If the Fix Works

Once confirmed working, you'll want to:
1. Push the changes to your egui fork on GitHub
2. Update the git URLs in `Cargo.toml` to point to your fork
3. Update `egui_commonmark` and `egui-twemoji` to use your egui fork
4. Test again with remote dependencies
5. Consider submitting a PR to upstream egui

