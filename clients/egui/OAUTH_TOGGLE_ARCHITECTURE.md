# egui OAuth Toggle Architecture

## Problem Statement

When using the egui client with Anthropic, users have two authentication modes:

1. **API Key Mode** - Pay-per-token, charges API balance
2. **Subscription Mode** - Uses Claude Pro/Max subscription via OAuth, no API charges

**The Bug:** The egui client was syncing API keys from `.env` on every connection, overwriting OAuth tokens in the server's `auth.json`, causing unexpected API charges even when the UI showed "(Subscription)".

## Solution Overview

Add a toggle in the egui UI to explicitly switch between Subscription (OAuth) and API Key modes, with live token expiration countdown and refresh capability.

---

## Architecture

### File Locations

**Server's auth storage:**

```
~/.local/share/opencode/auth.json
```

**egui's credential cache:**

```
clients/egui/.env
```

### auth.json Structure

The server's `auth.json` can store **ONE auth type per provider**:

**OAuth (Subscription) mode:**

```json
{
  "anthropic": {
    "type": "oauth",
    "access": "sk-ant-oat01-...", // Short-lived access token (~1 hour)
    "refresh": "sk-ant-ort01-...", // Long-lived refresh token
    "expires": 1767363684808 // Timestamp in milliseconds
  }
}
```

**API Key mode:**

```json
{
  "anthropic": {
    "type": "api",
    "key": "sk-ant-api03-..."
  }
}
```

**Important:** For the same provider, it's EITHER OAuth OR API key, not both simultaneously.

### egui's .env Cache

egui caches **BOTH** auth methods so it can switch between them:

```bash
# API Keys
ANTHROPIC_API_KEY=sk-ant-api03-...

# OAuth Tokens (cached from server's auth.json)
ANTHROPIC_OAUTH_ACCESS=sk-ant-oat01-...
ANTHROPIC_OAUTH_REFRESH=sk-ant-ort01-...
ANTHROPIC_OAUTH_EXPIRES=1767363684808
```

---

## OAuth Token Lifecycle

### How OAuth Tokens Are Generated

1. User runs: `opencode auth login`
2. CLI opens browser to Anthropic OAuth page
3. User authorizes, Anthropic shows authorization code:
   ```
   Byaf46lU3C5rizDbMrNkhcqOBuvor8zsDXDXtN6XFNi07sal#kobyLiU14RwcaALCEG2ChpoHNjlxVQgK3sPgAI3wVDENMWBdWwiI_Pt0YGpBhuMB0kuAY5th-OzWrE8Ak30myA
   ```
4. User pastes code into CLI
5. CLI exchanges code with Anthropic via `opencode-anthropic-auth@0.0.4` plugin
6. Anthropic returns:
   ```json
   {
     "access_token": "sk-ant-oat01-...",
     "refresh_token": "sk-ant-ort01-...",
     "expires_in": 3600 // seconds
   }
   ```
7. CLI calculates: `expires = Date.now() + expires_in * 1000`
8. CLI saves all three values to `~/.local/share/opencode/auth.json`

### Token Types

- **Access Token** (`sk-ant-oat01-...`): Used for API calls, expires in ~1 hour
- **Refresh Token** (`sk-ant-ort01-...`): Used to get new access tokens, long-lived (months/years)
- **Expires Timestamp**: When access token expires (milliseconds since epoch)

### Auto-Refresh Mechanism

The `opencode-anthropic-auth` plugin automatically refreshes tokens:

```javascript
if (!auth.access || auth.expires < Date.now()) {
  // Token expired, use refresh token to get new access token
  POST https://console.anthropic.com/v1/oauth/token
  Body: {
    grant_type: "refresh_token",
    refresh_token: auth.refresh,
    client_id: "9d1c250a-e61b-44d9-88ed-5944d1962f5e"
  }

  // Update auth.json with new tokens
}
```

---

## UI Design

### Layout (bottom-left panel)

```
[☑ Subscription (45m 23s)] [Model Dropdown ▼] [🔄]
```

**When Subscription mode OFF:**

```
[☐ Subscription] [Model Dropdown ▼]
```

(No countdown timer, no refresh button)

### Components

**1. Toggle Checkbox**

- Label: "Subscription"
- Checked = OAuth mode
- Unchecked = API Key mode
- State determined by reading `auth.json` on startup

**2. Countdown Timer**

- Only visible when Subscription mode is ON
- Format: `45m 23s` or `2h 15m` for longer durations
- Color-coded:
  - 🟢 Green: > 5 minutes remaining
  - 🟡 Yellow: < 5 minutes remaining
  - 🔴 Red: "⚠️ Expired"
- Updates every second (pure math, no I/O)

**3. Refresh Button (🔄)**

- Only visible when Subscription mode is ON
- Re-syncs OAuth tokens from server's `auth.json` to egui's `.env`
- Updates countdown timer with fresh expiration

---

## Implementation Details

### Startup Flow

```rust
// 1. Read server's auth.json
let auth_path = home_dir().join(".local/share/opencode/auth.json");
let auth_data: Value = serde_json::from_reader(File::open(auth_path)?)?;

// 2. Determine current auth mode
let is_subscription = match auth_data.get("anthropic") {
    Some(auth) if auth["type"] == "oauth" => {
        // Cache tokens to egui's .env if not already cached
        let access = auth["access"].as_str().unwrap();
        let refresh = auth["refresh"].as_str().unwrap();
        let expires = auth["expires"].as_u64().unwrap();

        write_to_env("ANTHROPIC_OAUTH_ACCESS", access);
        write_to_env("ANTHROPIC_OAUTH_REFRESH", refresh);
        write_to_env("ANTHROPIC_OAUTH_EXPIRES", expires);

        // Cache expires timestamp in memory for countdown
        self.oauth_expires = Some(expires);

        true
    }
    Some(auth) if auth["type"] == "api" => {
        self.oauth_expires = None;
        false
    }
    _ => {
        // No auth configured or invalid
        false
    }
};

// 3. Set toggle state
self.subscription_mode = is_subscription;
```

### Countdown Timer (Every Second)

```rust
impl OpenCodeApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Request repaint every second if in subscription mode
        if self.oauth_expires.is_some() {
            ctx.request_repaint_after(Duration::from_secs(1));
        }

        // ... rest of update logic
    }

    fn format_time_remaining(&self) -> String {
        if let Some(expires) = self.oauth_expires {
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64;

            if expires <= now {
                return "⚠️ Expired".to_string();
            }

            let remaining_ms = expires - now;
            let remaining_secs = remaining_ms / 1000;
            let minutes = remaining_secs / 60;
            let seconds = remaining_secs % 60;

            if minutes > 60 {
                let hours = minutes / 60;
                let mins = minutes % 60;
                format!("{}h {}m", hours, mins)
            } else {
                format!("{}m {}s", minutes, seconds)
            }
        } else {
            "N/A".to_string()
        }
    }
}
```

**Performance Note:** Zero I/O during countdown. The `expires` timestamp is cached in memory (`self.oauth_expires`), and we just do math every second to calculate remaining time.

### Toggle Click Handler

**When user clicks toggle ON (switch to Subscription):**

```rust
async fn switch_to_subscription(&mut self) -> Result<()> {
    // 1. Read cached OAuth tokens from egui's .env
    let access = env::var("ANTHROPIC_OAUTH_ACCESS")?;
    let refresh = env::var("ANTHROPIC_OAUTH_REFRESH")?;
    let expires = env::var("ANTHROPIC_OAUTH_EXPIRES")?.parse::<u64>()?;

    // 2. Check if token is expired
    let now = current_time_ms();
    if expires < now {
        show_error(
            "⚠️ No valid OAuth tokens cached.\n\
             Run: opencode auth login\n\
             Then click Refresh button."
        );
        return Err(TokenExpiredError);
    }

    // 3. Send OAuth tokens to server
    let client = reqwest::Client::new();
    client.put("http://localhost:4096/auth/anthropic")
        .json(&serde_json::json!({
            "type": "oauth",
            "access": access,
            "refresh": refresh,
            "expires": expires
        }))
        .send()
        .await?;

    // 4. Reload server state
    client.post("http://localhost:4096/instance/dispose")
        .send()
        .await?;

    // 5. Update UI state
    self.subscription_mode = true;
    self.oauth_expires = Some(expires);

    show_toast("✓ Switched to Subscription mode");
    Ok(())
}
```

**When user clicks toggle OFF (switch to API Key):**

```rust
async fn switch_to_api_key(&mut self) -> Result<()> {
    // 1. Read API key from egui's .env
    let api_key = env::var("ANTHROPIC_API_KEY")?;

    // 2. Send API key to server
    let client = reqwest::Client::new();
    client.put("http://localhost:4096/auth/anthropic")
        .json(&serde_json::json!({
            "type": "api",
            "key": api_key
        }))
        .send()
        .await?;

    // 3. Reload server state
    client.post("http://localhost:4096/instance/dispose")
        .send()
        .await?;

    // 4. Update UI state
    self.subscription_mode = false;
    self.oauth_expires = None;

    show_toast("✓ Switched to API Key mode");
    Ok(())
}
```

### Refresh Button Handler

```rust
async fn refresh_oauth_tokens(&mut self) -> Result<()> {
    // 1. Read server's auth.json
    let auth_path = home_dir().join(".local/share/opencode/auth.json");
    let auth_data: Value = serde_json::from_reader(File::open(auth_path)?)?;

    // 2. Extract Anthropic OAuth data
    let anthropic = auth_data["anthropic"].as_object()
        .ok_or(anyhow!("No Anthropic auth found"))?;

    if anthropic["type"] != "oauth" {
        show_error("Server is not in OAuth mode. Run: opencode auth login");
        return Err(NotOAuthError);
    }

    let access = anthropic["access"].as_str().unwrap();
    let refresh = anthropic["refresh"].as_str().unwrap();
    let expires = anthropic["expires"].as_u64().unwrap();

    // 3. Update egui's .env cache
    write_to_env("ANTHROPIC_OAUTH_ACCESS", access);
    write_to_env("ANTHROPIC_OAUTH_REFRESH", refresh);
    write_to_env("ANTHROPIC_OAUTH_EXPIRES", expires);

    // 4. Update in-memory cached timestamp
    self.oauth_expires = Some(expires);

    show_toast("✓ Subscription tokens refreshed");
    Ok(())
}
```

---

## Server Endpoints

### PUT /auth/:id

Sets authentication credentials for a provider.

**Request:**

```http
PUT /auth/anthropic
Content-Type: application/json

{
  "type": "oauth",
  "access": "sk-ant-oat01-...",
  "refresh": "sk-ant-ort01-...",
  "expires": 1767363684808
}
```

**Or:**

```http
PUT /auth/anthropic
Content-Type: application/json

{
  "type": "api",
  "key": "sk-ant-api03-..."
}
```

**Response:**

```json
true
```

**Implementation:** Updates `~/.local/share/opencode/auth.json` via `Auth.set(id, info)`

### POST /instance/dispose

Disposes the current provider instance, forcing it to reload on next request.

**Request:**

```http
POST /instance/dispose
```

**Response:**

```json
true
```

**Effect:** Clears cached provider state, causing server to re-read `auth.json` and reload provider configuration (including new auth credentials).

---

## User Workflows

### Workflow 1: Initial Setup (New User)

1. User installs OpenCode
2. User runs: `opencode auth login`
3. Selects "Anthropic" → "Claude Pro/Max"
4. Pastes authorization code
5. OAuth tokens saved to `~/.local/share/opencode/auth.json`
6. User starts egui: `./opencode-egui`
7. egui reads `auth.json`, sees OAuth, caches tokens
8. Toggle shows: `[☑ Subscription (58m 42s)]`

### Workflow 2: Switching to API Key

1. User clicks toggle OFF
2. egui sends API key to server
3. Server updates `auth.json` to `type: "api"`
4. Server reloads provider state
5. Toggle shows: `[☐ Subscription]`
6. Requests now use API key, charge API balance

### Workflow 3: Token Expired, Need Refresh

1. User sees: `[☑ Subscription (⚠️ Expired)]`
2. User runs in terminal: `opencode auth login`
3. Completes OAuth flow (new tokens saved)
4. User clicks **🔄 Refresh** button in egui
5. egui syncs new tokens from `auth.json` to `.env`
6. Countdown updates: `[☑ Subscription (59m 58s)]`

### Workflow 4: Daily Usage

1. User starts egui (server already running)
2. egui shows current mode from `auth.json`
3. If subscription: countdown shows time remaining
4. User works normally
5. If token expires during session: warning appears
6. User refreshes via CLI + Refresh button

---

## Edge Cases & Error Handling

### Case 1: No Auth Configured

**On startup:**

```
⚠️ No Anthropic authentication configured.
Run: opencode auth login
```

Toggle is disabled/grayed out until auth is set up.

### Case 2: OAuth Token Expired on Startup

**On startup:**

```
⚠️ Anthropic OAuth token expired (0h 0m).
Run: opencode auth login
Then click Refresh button.
```

Toggle shows checked but with expired warning.

### Case 3: Switching to Subscription with Expired Cache

**When clicking toggle ON:**

```
⚠️ No valid OAuth tokens cached.
Run: opencode auth login
Then click Refresh button.
```

Toggle reverts to OFF state.

### Case 4: Server auth.json Changed Externally

**Problem:** User runs `opencode auth login` in terminal while egui is running, but doesn't click Refresh.

**Solution:** egui continues using cached tokens. Countdown still shows old expiration. User must click Refresh to sync.

**Alternative (optional):** egui watches `auth.json` for changes and auto-syncs. This adds complexity and file watching overhead.

### Case 5: Server Not Running

**When clicking toggle:**

```
❌ Cannot connect to OpenCode server (http://localhost:4096)
Is the server running?
```

Toggle reverts to previous state.

---

## Security Considerations

### File Permissions

**Server's auth.json:**

- Location: `~/.local/share/opencode/auth.json`
- Permissions: `0600` (user read/write only)
- Set by: `Auth.set()` in server code

**egui's .env:**

- Location: `clients/egui/.env`
- Permissions: Should be `0600` (user read/write only)
- Gitignored: Yes (via `.gitignore`)

### Token Exposure

**Risks:**

- OAuth tokens visible in `.env` file
- Access tokens short-lived (1 hour), limited risk
- Refresh tokens long-lived, higher risk if exposed

**Mitigations:**

- File permissions restrict to user only
- `.env` in gitignore prevents accidental commit
- egui reads from memory after startup (no repeated file reads)

### Network Security

**API Calls:**

- Server runs on `localhost:4096` by default
- egui → server communication over local loopback
- No external network exposure

**OAuth Flow:**

- Handled by `opencode` CLI, not egui
- Uses PKCE (Proof Key for Code Exchange) for security
- Authorization code is single-use

---

## Testing Plan

### Manual Tests

1. **Fresh Install Test**
   - Clean install, no auth configured
   - Start egui, verify toggle is OFF
   - Run `opencode auth login` (API key)
   - Restart egui, verify toggle is OFF
   - Run `opencode auth login` (OAuth)
   - Restart egui, verify toggle is ON with countdown

2. **Toggle Switch Test**
   - Start in OAuth mode
   - Click toggle OFF → verify API key sent, server reloaded
   - Make request → verify API balance changes
   - Click toggle ON → verify OAuth sent, server reloaded
   - Make request → verify API balance doesn't change

3. **Countdown Timer Test**
   - Start in OAuth mode with valid token
   - Verify countdown updates every second
   - Verify color changes (green → yellow → red)
   - Wait for expiration → verify "Expired" warning

4. **Refresh Button Test**
   - Start with expired OAuth token
   - Run `opencode auth login` to get fresh tokens
   - Click Refresh → verify countdown updates
   - Verify `.env` file updated

5. **Error Handling Test**
   - Stop OpenCode server
   - Try to toggle → verify error message
   - Start server, retry → verify success

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_time_remaining_minutes() {
        let app = OpenCodeApp {
            oauth_expires: Some(current_time_ms() + 2700000), // 45 minutes
            ..Default::default()
        };
        let formatted = app.format_time_remaining();
        assert_eq!(formatted, "45m 0s");
    }

    #[test]
    fn test_format_time_remaining_hours() {
        let app = OpenCodeApp {
            oauth_expires: Some(current_time_ms() + 7500000), // 2h 5m
            ..Default::default()
        };
        let formatted = app.format_time_remaining();
        assert_eq!(formatted, "2h 5m");
    }

    #[test]
    fn test_format_time_remaining_expired() {
        let app = OpenCodeApp {
            oauth_expires: Some(current_time_ms() - 1000), // 1 second ago
            ..Default::default()
        };
        let formatted = app.format_time_remaining();
        assert_eq!(formatted, "⚠️ Expired");
    }

    #[test]
    fn test_parse_auth_json_oauth() {
        let json = r#"{
            "anthropic": {
                "type": "oauth",
                "access": "sk-ant-oat01-test",
                "refresh": "sk-ant-ort01-test",
                "expires": 1767363684808
            }
        }"#;

        let auth: Value = serde_json::from_str(json).unwrap();
        assert_eq!(auth["anthropic"]["type"], "oauth");
        assert_eq!(auth["anthropic"]["access"], "sk-ant-oat01-test");
    }

    #[test]
    fn test_parse_auth_json_api() {
        let json = r#"{
            "anthropic": {
                "type": "api",
                "key": "sk-ant-api03-test"
            }
        }"#;

        let auth: Value = serde_json::from_str(json).unwrap();
        assert_eq!(auth["anthropic"]["type"], "api");
        assert_eq!(auth["anthropic"]["key"], "sk-ant-api03-test");
    }
}
```

---

## Future Enhancements

### Auto-Refresh on Expiration

Currently, when the token expires, the user must:

1. Run `opencode auth login` in terminal
2. Click Refresh button in egui

**Enhancement:** egui could automatically trigger refresh using the refresh token when expiration approaches.

**Pros:**

- Seamless UX, no manual intervention
- No service interruption

**Cons:**

- egui needs to implement OAuth token refresh logic
- Adds complexity to egui client
- Refresh token could fail (network, invalid token)

**Decision:** Defer to future. Current manual flow is acceptable for MVP.

### File Watching

Watch `~/.local/share/opencode/auth.json` for changes and auto-sync to egui's `.env`.

**Pros:**

- Auto-detects when user runs `opencode auth login`
- No need to click Refresh button

**Cons:**

- Adds file watching overhead
- Cross-platform compatibility (inotify, FSEvents, etc.)
- Potential race conditions

**Decision:** Defer to future. Manual Refresh button is simpler for MVP.

### Multiple Provider Support

Extend toggle to support other providers (e.g., OpenAI if they add subscription plans).

**Design:**

- One toggle per provider that supports both auth types
- Could be a dropdown instead: `[Anthropic: Subscription ▼]`

**Decision:** Out of scope. Only Anthropic has subscription plans currently.

---

## References

### Key Files

**egui Client:**

- `clients/egui/src/app.rs` - Main app state and UI
- `clients/egui/src/startup/auth.rs` - Auth syncing logic
- `clients/egui/.env` - Credential cache

**OpenCode Server:**

- `packages/opencode/src/auth/index.ts` - Auth storage interface
- `packages/opencode/src/server/server.ts` - HTTP endpoints
- `packages/opencode/src/provider/provider.ts` - Provider loading
- `~/.local/share/opencode/auth.json` - Auth storage

**Plugin:**

- `opencode-anthropic-auth@0.0.4` (npm package)
- Handles OAuth flow and token refresh

### API Endpoints

- `PUT /auth/:id` - Set auth credentials (line 2106-2135 in server.ts)
- `POST /instance/dispose` - Reload provider state (line ~2000 in server.ts)
- `GET /provider` - List providers with connection status (line 1377-1409 in server.ts)

### Environment Variables

```bash
# Anthropic API Key (from .env.example)
ANTHROPIC_API_KEY=sk-ant-api03-...

# OpenAI API Key
OPENAI_API_KEY=sk-proj-...

# Google AI API Key
GOOGLE_API_KEY=AIza...

# OpenRouter API Key
OPENROUTER_API_KEY=sk-or-v1-...

# OAuth Tokens (added by this feature)
ANTHROPIC_OAUTH_ACCESS=sk-ant-oat01-...
ANTHROPIC_OAUTH_REFRESH=sk-ant-ort01-...
ANTHROPIC_OAUTH_EXPIRES=1767363684808
```

---

## Glossary

**OAuth** - Open Authorization, an authorization framework that enables applications to obtain limited access to user accounts

**Access Token** - Short-lived token used to authenticate API requests (expires in ~1 hour)

**Refresh Token** - Long-lived token used to obtain new access tokens without re-authenticating

**PKCE** - Proof Key for Code Exchange, a security extension to OAuth to prevent authorization code interception

**Authorization Code** - Single-use code obtained from OAuth provider, exchanged for access/refresh tokens

**Instance.dispose** - Server operation that clears cached provider state and forces reload

**auth.json** - Server's authentication credential storage file

**.env** - egui's environment variable file for caching credentials

**Subscription Mode** - Using Anthropic via Claude Pro/Max subscription (no API charges)

**API Key Mode** - Using Anthropic via pay-per-token API key (charges API balance)

---

**Document Version:** 1.0  
**Last Updated:** 2025-01-02  
**Authors:** Tony & Claude (OpenCode AI)
