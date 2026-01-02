# Anthropic OAuth Subscription Detection - Complete Summary

**Date**: January 2, 2025  
**Branch**: `feature/anthropic-subscription`  
**Status**: ✅ Code complete, ready for testing

---

## What We Did

Added visual indicator to egui client that shows when you're using your Anthropic Pro/Max subscription (via OAuth) instead of API keys.

**Result**: Model selector in bottom-left shows `🟢 Claude Sonnet (Subscription)` when OAuth is active.

---

## The Problem We Solved

GPT initially implemented a broken OAuth flow where:

- egui client would run OAuth and save tokens locally
- egui would send tokens to server via `Authorization` header
- **Server ignored this header completely**
- Nothing worked

We discovered:

1. Server already has working OAuth via CLI: `opencode auth login`
2. Server exposes `GET /provider` endpoint with `connected: ["anthropic"]` array
3. All we need to do is **query the server** and **show an indicator**

---

## Authentication Status (Verified Working)

Your server's `~/.local/share/opencode/auth.json` contains:

```json
{
  "anthropic": {
    "type": "oauth",
    "refresh": "sk-ant-ort01-...",
    "access": "sk-ant-oat01-...",
    "expires": 1767360094945
  }
}
```

✅ **OAuth is configured and working** (expires Jan 2, 2026)

The `opencode-anthropic-auth` plugin automatically:

- Reads these tokens from server's auth.json
- Adds `Authorization: Bearer <token>` to Anthropic API calls
- Auto-refreshes expired tokens
- Sets model costs to $0 (subscription mode)

---

## Code Changes (4 files, ~40 new lines)

### 1. `src/client/api.rs` (+15 lines)

Added method to query server:

```rust
pub async fn get_provider_status(&self) -> Result<ProviderStatus, ApiError> {
    let url = self.base.join("provider")?;
    let resp = self.prepare_request(self.http.get(url)).send().await?;
    resp.json().await.map_err(|e| ApiError::Decode(e.to_string()))
}

#[derive(Debug, Clone, Deserialize)]
pub struct ProviderStatus {
    pub connected: Vec<String>,
}
```

### 2. `src/app.rs` (+25 lines)

Added state tracking:

```rust
pub struct OpenCodeApp {
    // ... existing fields ...
    connected_providers: Vec<String>,  // NEW
}
```

Added message type:

```rust
enum UiMsg {
    // ... existing variants ...
    ProviderStatus(Vec<String>),  // NEW
}
```

Fetch status on connection (around line 367):

```rust
if let (Some(client), Some(rt)) = (&self.client, &self.runtime) {
    let client_clone = client.clone();
    let tx = self.ui_tx.as_ref().unwrap().clone();
    let egui_ctx = ctx.clone();
    rt.spawn(async move {
        if let Ok(status) = client_clone.get_provider_status().await {
            let _ = tx.send(UiMsg::ProviderStatus(status.connected));
            egui_ctx.request_repaint();
        }
    });
}
```

Handle message (around line 687):

```rust
UiMsg::ProviderStatus(connected) => {
    self.connected_providers = connected;
}
```

Update model selector display (around line 2724):

```rust
let current_display = if let Some((provider, model_id)) = &tab.selected_model {
    let is_oauth = self.connected_providers.contains(provider);
    let base_display = /* lookup model name */;

    if is_oauth && provider == "anthropic" {
        format!("🟢 {} (Subscription)", base_display)
    } else {
        base_display
    }
} else {
    if self.connected_providers.contains(&"anthropic".to_string()) {
        "🟢 (Anthropic Subscription)".to_string()
    } else {
        "(default)".to_string()
    }
};
```

### 3. `src/main.rs` (-2 lines)

Removed unused imports:

```rust
// DELETED: use std::io::{self, Write};
```

### 4. Initialize state in `new()`

Added to constructor:

```rust
connected_providers: Vec::new(),
```

---

## How It Works

```
1. User runs CLI: opencode auth login
   ↓
2. Server saves OAuth tokens to ~/.local/share/opencode/auth.json
   ↓
3. User starts egui client
   ↓
4. egui calls: GET http://127.0.0.1:8080/provider
   ↓
5. Server returns: { "connected": ["anthropic"] }
   ↓
6. egui detects "anthropic" in list
   ↓
7. Model selector shows: 🟢 Claude Sonnet (Subscription)
```

---

## Testing (NOT DONE YET)

**To test**:

```bash
cd /Users/tony/git/opencode/clients/egui
cargo run
```

**Expected**: Bottom-left model dropdown shows `🟢 Claude Sonnet (Subscription)`

**If it doesn't work, check**:

1. Server running?

   ```bash
   ps aux | grep opencode
   ```

2. OAuth configured?

   ```bash
   cat ~/.local/share/opencode/auth.json | jq .anthropic
   ```

3. Server responds?
   ```bash
   curl http://127.0.0.1:8080/provider | jq .connected
   # Should include "anthropic"
   ```

---

## Compilation Status

✅ **Compiles clean**:

```bash
cargo check
# Finished `dev` profile [unoptimized + debuginfo]
```

✅ **All tests pass**:

```bash
cargo test
# test result: ok. 14 passed; 0 failed
```

---

## Git Status

**Modified** (10 files):

- `Cargo.toml` / `Cargo.lock` - Dependencies (clap, reqwest blocking)
- `src/app.rs` - Status detection and UI display
- `src/client/api.rs` - Provider status API call
- `src/main.rs` - Fixed unused imports
- Plus 6 other files with GPT's broken OAuth code

**Untracked**:

- `src/tests/auth_oauth.rs` - OAuth tests (part of broken implementation)

**Branch**: `feature/anthropic-subscription`

---

## Dead Code (Needs Cleanup Later)

The following code was added by GPT but **DOES NOT WORK** and should be deleted:

1. `src/startup/auth.rs` (lines 140-325) - OAuth flow implementation
2. `src/main.rs` - `--oauth` flag and `run_oauth_flow()` function
3. `src/app.rs` - `oauth_token` field and loading logic
4. `src/client/api.rs` - `oauth_token` field, `set_oauth_token()`, Authorization header
5. `src/tests/auth_oauth.rs` - OAuth tests
6. `README.md` - `--oauth` documentation

**Total**: ~200 lines of dead code

**Why it's broken**: egui tries to send OAuth token to server via header, but server never reads that header. Server only reads from its own `auth.json` file.

---

## Architecture (How OAuth Actually Works)

```
┌─────────────┐
│    User     │
└──────┬──────┘
       │ opencode auth login
       ▼
┌─────────────────────────┐
│  OpenCode CLI/Server    │
│  Runs OAuth flow        │
└──────┬──────────────────┘
       │ Saves tokens
       ▼
┌─────────────────────────┐
│ ~/.local/share/         │
│  opencode/auth.json     │
└──────┬──────────────────┘
       │ Plugin reads
       ▼
┌─────────────────────────┐
│ opencode-anthropic-     │
│  auth plugin            │
│ - Auto-refresh tokens   │
│ - Add Bearer header     │
│ - Zero out costs        │
└──────┬──────────────────┘
       │ Make API calls
       ▼
┌─────────────────────────┐
│   Anthropic API         │
│ (subscription quota)    │
└─────────────────────────┘

       ┌─────────────┐
       │ egui Client │
       └──────┬──────┘
              │ GET /provider
              ▼
       ┌─────────────────────┐
       │  OpenCode Server    │
       │  Returns connected: │
       │   ["anthropic"]     │
       └──────┬──────────────┘
              │ Update UI
              ▼
       ┌─────────────────────┐
       │  Model Selector     │
       │  🟢 (Subscription)  │
       └─────────────────────┘
```

**Key point**: egui NEVER handles OAuth tokens. It just asks the server "what's authenticated?" and displays the result.

---

## What the Indicator Looks Like

**Before this change**:

```
┌─────────────────────────┐
│ Claude Sonnet          ▼│
└─────────────────────────┘
```

**After (with OAuth)**:

```
┌──────────────────────────────────────┐
│ 🟢 Claude Sonnet (Subscription)    ▼│
└──────────────────────────────────────┘
```

**After (no model selected, OAuth active)**:

```
┌──────────────────────────────────────┐
│ 🟢 (Anthropic Subscription)        ▼│
└──────────────────────────────────────┘
```

---

## Next Session Context

**When you come back**:

1. **Test the egui client**: `cargo run` and verify green circle appears
2. **If it works**: Celebrate, then delete the ~200 lines of dead OAuth code
3. **If it doesn't work**: Check the debug steps above, verify server endpoint
4. **After testing**: Clean up, commit, document the correct CLI workflow

**Current state**: Code is done, compiles, tests pass, but NOT TESTED in the actual GUI yet.

---

## Quick Reference

**Start server**:

```bash
./packages/opencode/dist/opencode-darwin-arm64/bin/opencode serve
```

**Start egui**:

```bash
cd clients/egui
cargo run
```

**Check OAuth status**:

```bash
cat ~/.local/share/opencode/auth.json | jq .anthropic.type
# Should output: "oauth"
```

**Check server endpoint**:

```bash
curl http://127.0.0.1:8080/provider | jq .connected
# Should include: "anthropic"
```

---

## Summary

- ✅ **OAuth is configured** (via CLI, already done)
- ✅ **Code is complete** (~40 new lines)
- ✅ **Compiles and tests pass**
- ⏳ **Not tested in GUI yet** (need to restart and run egui)
- 🗑️ **~200 lines of dead code to delete later** (GPT's broken OAuth flow)

**The implementation is simple**: Query server for authenticated providers, show green circle if "anthropic" is in the list. That's it.
