# Session Plan: Anthropic OAuth Implementation for egui Client

## Goal

Add Anthropic Pro/Max OAuth support to egui OpenCode client so you can use your Claude subscription instead of paying $300/month for API access.

---

## Session 1: Foundation & OAuth Data Types

### Step 1: Add Dependencies

- Add `clap` for CLI argument parsing (`--oauth` flag)
- Add `reqwest` (blocking) for HTTP requests
- Add `serde` + `serde_json` for JSON handling
- Update `Cargo.toml`

**Status:** ⏳ Pending

---

### Step 2: Create OAuth Data Types

- Create `src/startup/auth.rs` (or similar)
- Define structs:
  - `DeviceCodeRequest` - POST to `/oauth/device/code`
  - `DeviceCodeResponse` - Contains `device_code`, `user_code`, `verification_uri`
  - `TokenRequest` - POST to `/oauth/token` for polling
  - `TokenResponse` - Contains `access_token`, `refresh_token`, `expires_in`
  - `OAuthError` - Custom error type
- Implement proper error handling (From traits, etc.)
- Add unit tests for serialization/deserialization

**Status:** ⏳ Pending

---

### Step 3: Implement Device Code Flow (Part 1)

- Function: `request_device_code() -> Result<DeviceCodeResponse, OAuthError>`
- POST to `https://api.anthropic.com/oauth/device/code`
- Body: `{ "client_id": "your-client-id", "scope": "api" }`
- Parse response
- Handle errors properly

**Status:** ⏳ Pending

---

### Step 4: Display User Code & Poll for Token

- Function: `poll_for_token(device_code: &str, interval: u64) -> Result<TokenResponse, OAuthError>`
- Print user instructions: "Go to {verification_uri} and enter code: {user_code}"
- Poll POST to `/oauth/token` every `interval` seconds
- Stop polling when token received or timeout/error
- Handle `authorization_pending` vs real errors

**Status:** ⏳ Pending

---

### Step 5: Save Token to Disk

- Function: `save_token(token: TokenResponse) -> Result<(), OAuthError>`
- Save to `~/.local/share/opencode/auth.json` (match main OpenCode location)
- Format: `{ "access_token": "...", "refresh_token": "...", "expires_at": <unix_timestamp> }`
- Create directory if doesn't exist
- Proper file permissions (600)

**Status:** ⏳ Pending

---

### Step 6: Session Summary

- Document what was completed
- Document any blockers or issues
- Update this file with progress
- Prepare handoff for Session 2

**Status:** ⏳ Pending

---

## Session 2: CLI Integration & Testing

### Step 7: Add CLI Parsing

- Update `src/main.rs`
- Add `clap` CLI parser with `--oauth` flag
- Parse arguments before GUI launch
- Decide: run OAuth flow or normal startup

**Status:** ⏳ Pending

---

### Step 8: Wire OAuth Flow to Startup

- If `--oauth` flag:
  1. Run device code flow
  2. Poll for token (blocking)
  3. Save token to disk
  4. Exit (or continue to GUI with OAuth context)
- If no flag: normal API key startup
- Add proper logging/output for user

**Status:** ⏳ Pending

---

### Step 9: Handle OAuth Context in App

- Check if OAuth token exists at startup
- Load token from `auth.json`
- Pass OAuth context to app state
- Ensure API requests use OAuth token (Authorization: Bearer header)
- Add token refresh logic (optional for MVP)

**Status:** ⏳ Pending

---

### Step 10: Model Pre-Selection (Optional UX)

- When using OAuth, auto-select "Claude Sonnet" or similar
- Update model selector UI to show OAuth context
- Ensure user can still switch models freely

**Status:** ⏳ Pending

---

### Step 11: Testing & Validation

- Test full OAuth flow from CLI
- Test normal API key flow still works
- Test token persistence across app restarts
- Test error handling (invalid token, network errors, etc.)
- Verify API requests use correct authentication

**Status:** ⏳ Pending

---

### Step 12: Final Documentation

- Update README with OAuth usage instructions
- Document CLI flags and workflow
- Add troubleshooting section
- Clean up any temporary files

**Status:** ⏳ Pending

---

## Success Criteria

### Session 1

- [ ] Dependencies added and compiling
- [ ] OAuth data types defined with tests
- [ ] Device code flow implemented
- [ ] Token polling works
- [ ] Token saving to disk works

### Session 2

- [ ] CLI parsing with `--oauth` flag works
- [ ] OAuth flow triggers correctly from CLI
- [ ] Token persists and loads on restart
- [ ] API requests use OAuth token
- [ ] Both OAuth and API key flows work
- [ ] Model pre-selection works (optional)

---

## Notes & Decisions

### Why This Approach?

- **CLI-based OAuth:** Simpler than GUI, device code flow is designed for CLIs
- **Blocking flow:** Happens before GUI, no async complexity
- **Token persistence:** Reuses OpenCode's `auth.json` pattern
- **Backwards compatible:** Existing API key users unaffected

### Key Technical Details

- **models.dev API:** Used for model discovery, no auth required
- **Token location:** `~/.local/share/opencode/auth.json`
- **Device code flow:** Show code → user authorizes → poll for token
- **OAuth endpoints:**
  - Device code: `POST https://api.anthropic.com/oauth/device/code`
  - Token: `POST https://api.anthropic.com/oauth/token`

### Files to Create/Modify

- Create: `src/startup/auth.rs` (OAuth logic)
- Modify: `src/main.rs` (CLI parsing, startup flow)
- Modify: `src/app.rs` (OAuth context, API calls)
- Modify: `Cargo.toml` (dependencies)

### Dependencies

```toml
[dependencies]
clap = { version = "4", features = ["derive"] }
reqwest = { version = "0.11", features = ["blocking", "json"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
```

---

## Session History

### Session 1 (Not Started)

- Status: ⏳ Awaiting start
- Next Step: Add dependencies and create OAuth data types

---

## What Went Wrong Last Time

**I wasted 200K tokens creating:**

- 15 overlapping documentation files
- Broken references to files that don't exist
- Over-engineered navigation maze
- Zero actual implementation

**This time:**

- Two files: `SESSION_PLAN.md` (this), `NEXT_SESSION_PROMPT.md` (handoff)
- Clear task breakdown with checkboxes
- No broken references
- No fluff

---

## Token Budget Strategy

**Goal:** Don't hit 200K mid-implementation

**Session 1 estimate:** ~80-120K tokens

- Reading files
- Writing OAuth logic
- Tests
- Debugging

**Session 2 estimate:** ~60-80K tokens

- CLI integration
- Testing
- Documentation

**Total:** ~140-200K tokens across 2 sessions

**If Session 1 hits ~150K:** Stop, summarize, start Session 2 fresh
