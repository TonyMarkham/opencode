# Next Session: Implement Anthropic OAuth (Session 1)

## Quick Context

**What You Want:**

- Add Anthropic Pro/Max OAuth to egui OpenCode client
- Use your Claude subscription instead of $300/month API costs
- Get access to Opus via Max 20x plan ($200)

**Current State:**

- Planning complete, zero implementation done
- Previous session wasted 200K tokens on broken documentation
- This time: TWO files, clear tasks, no fluff

---

## Your Mission: Session 1

Build the **OAuth Foundation & Device Code Flow**.

**Work from:** `/Users/tony/git/opencode/clients/egui/SESSION_PLAN.md`

**Complete:** Steps 1-6 (Foundation through first session summary)

---

## What You'll Build

### 1. Add Dependencies (5 min)

Add to `Cargo.toml`:

```toml
clap = { version = "4", features = ["derive"] }
reqwest = { version = "0.11", features = ["blocking", "json"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
```

### 2. Create OAuth Data Types (30 min)

Create `src/startup/auth.rs` with:

- `DeviceCodeRequest` / `DeviceCodeResponse`
- `TokenRequest` / `TokenResponse`
- `OAuthError` custom error type
- Proper error handling (From traits)
- Unit tests for JSON serialization

### 3. Implement Device Code Request (20 min)

Function: `request_device_code() -> Result<DeviceCodeResponse, OAuthError>`

- POST to `https://api.anthropic.com/oauth/device/code`
- Body: `{ "client_id": "your-client-id", "scope": "api" }`
- Return parsed response

### 4. Implement Token Polling (30 min)

Function: `poll_for_token(device_code: &str, interval: u64) -> Result<TokenResponse, OAuthError>`

- Print: "Go to {verification_uri} and enter code: {user_code}"
- Loop: POST to `/oauth/token` every `interval` seconds
- Stop on success or error (not `authorization_pending`)

### 5. Save Token to Disk (20 min)

Function: `save_token(token: TokenResponse) -> Result<(), OAuthError>`

- Path: `~/.local/share/opencode/auth.json`
- Format: `{ "access_token": "...", "refresh_token": "...", "expires_at": <timestamp> }`
- Create dirs if needed, set permissions 600

### 6. Session Summary (10 min)

- Update SESSION_PLAN.md with checkboxes
- Note any issues or blockers
- Update NEXT_SESSION_PROMPT.md for Session 2

**Total time estimate: ~2 hours**

---

## Key Technical Details

**OAuth Endpoints:**

```
POST https://api.anthropic.com/oauth/device/code
POST https://api.anthropic.com/oauth/token
```

**Device Code Flow:**

1. POST to `/device/code` → get `device_code` + `user_code`
2. Show user: "Go to URL and enter code"
3. Poll `/token` every N seconds until token received
4. Save token to disk

**Token Storage:**

- Path: `~/.local/share/opencode/auth.json`
- Schema:

```json
{
  "access_token": "sk-ant-...",
  "refresh_token": "refresh-...",
  "expires_at": 1735689600
}
```

---

## Files You'll Create/Modify

**Create:**

- `src/startup/auth.rs` (all OAuth logic)

**Modify:**

- `Cargo.toml` (dependencies)

**Don't Touch Yet:**

- `src/main.rs` (CLI parsing - Session 2)
- `src/app.rs` (OAuth context - Session 2)

---

## Success Criteria

By end of Session 1:

- [ ] Dependencies compile
- [ ] `auth.rs` module exists with all data types
- [ ] Device code request works (tested manually or via test)
- [ ] Token polling loop works
- [ ] Token saves to disk correctly
- [ ] Unit tests pass

---

## Copy-Paste Starting Prompt

Use this when starting Session 1:

```
I'm implementing Anthropic OAuth for the egui OpenCode client.

Work from: /Users/tony/git/opencode/clients/egui/SESSION_PLAN.md
Next steps: /Users/tony/git/opencode/clients/egui/NEXT_SESSION_PROMPT.md

Complete Session 1, Steps 1-6:
1. Add dependencies (clap, reqwest, serde)
2. Create src/startup/auth.rs with OAuth data types
3. Implement device code request function
4. Implement token polling function
5. Implement token save to disk
6. Update SESSION_PLAN.md with progress

Start with Step 1: Add dependencies to Cargo.toml
```

---

## Important Reminders

1. **Test as you go** - Don't write all code then test
2. **Update SESSION_PLAN.md** - Check boxes as you complete steps
3. **Watch token usage** - If approaching 150K, stop and summarize
4. **No scope creep** - Session 1 is foundation only, no CLI integration yet
5. **Blocking reqwest** - Use `reqwest::blocking` not async

---

## If You Get Stuck

**Can't find client_id?**

- Check OpenCode docs or main OpenCode repo
- May need to register OAuth app with Anthropic
- Placeholder: `"opencode-egui"` for testing

**Token format unclear?**

- Match main OpenCode's `auth.json` format
- Check `/packages/opencode/src/provider/auth.ts` for reference

**reqwest errors?**

- Ensure using `reqwest::blocking::Client`
- Add proper error handling with `?` operator
- Check Content-Type header: `application/json`

---

## What NOT to Do

- ❌ Don't create 15 documentation files
- ❌ Don't implement CLI parsing yet (that's Session 2)
- ❌ Don't touch GUI code
- ❌ Don't add async/tokio (use blocking reqwest)
- ❌ Don't implement token refresh (MVP later)

---

## Session 2 Preview

After Session 1 completes:

- Add CLI parsing (`--oauth` flag)
- Wire OAuth flow to `main.rs` startup
- Handle OAuth context in app state
- Test full flow end-to-end
- Optional: Model pre-selection UI

**But don't think about Session 2 yet. Focus on Session 1.**

---

## Quick Start Checklist

Before coding:

- [ ] Read SESSION_PLAN.md fully
- [ ] Read this file fully
- [ ] Understand device code flow
- [ ] Have editor open to `clients/egui/`
- [ ] Know where `Cargo.toml` is

Then:

- [ ] Copy starting prompt above
- [ ] Start new Claude session
- [ ] Begin Step 1

---

**Ready? Start Session 1 now.**
