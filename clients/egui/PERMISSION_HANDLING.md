# Feature: Permission Request Handling in EGUI Client

## Status: ✅ COMPLETED (2025-12-08)

**Implementation Summary**:
- Inline permission UI integrated into tool call bubbles
- Auto-expands collapsible header when permission is pending
- Session-level prompt gating (blocks input until permission resolved)
- Generic handling of all permission types (external_directory, bash, write, etc.)
- Feature parity with TUI client achieved

**Key Implementation Details**:
- Permission approval UI renders inline within the tool's chat bubble (not modal)
- Collapsible tool section auto-expands when permission is pending
- Matches permissions to tool calls via `call_id` and `session_id`
- Three response buttons: ❌ Reject, ✅ Allow Once, ✅ Always Allow
- Input prompt disabled while any permission is pending in the session
- No debug logging spam (removed after validation)

---

## Problem

The OpenCode EGUI client currently does not handle permission requests from the server. When the agent attempts actions that require user approval (accessing external directories, executing commands, fetching URLs, etc.), the server blocks waiting for a response that never comes.

**Current Behavior**:
- Tool calls appear "stuck" indefinitely in the UI
- No visual indication that permission is required
- No way to approve or reject the request
- User must force-close the session or restart

**Example Stuck Scenario**:
```
User: "Can you summarize /Users/tony/git/bmad-engine/README.md"
Agent: [calls read tool with external file path]
Server: [blocks at Permission.ask(), waiting for response]
EGUI: [shows spinning tool call indicator forever]
```

## Root Cause

The permission flow requires client interaction:

1. **Server Side** (`packages/opencode/src/permission/index.ts`):
   - Tool calls `Permission.ask()` when action requires approval
   - Publishes `permission.updated` event via SSE
   - Blocks execution until `Permission.respond()` is called
   - Will wait indefinitely if no response received

2. **Client Side** (EGUI):
   - Does NOT subscribe to permission events
   - Does NOT display permission dialog UI
   - Does NOT send permission responses

## Permission Types

The following actions can trigger permission requests (when configured with `"ask"`):

| Permission Type | Triggered By | Example |
|----------------|--------------|---------|
| `external_directory` | read, write, edit, patch tools | Reading `/other/repo/file.txt` |
| `bash` | bash tool | Running restricted commands |
| `webfetch` | webfetch tool | Fetching external URLs |
| `websearch` | websearch tool | Performing web searches |

## Server API

### SSE Event: `permission.updated`

Sent when permission is requested:

```json
{
  "id": "permission_abc123",
  "type": "external_directory",
  "pattern": ["/Users/tony/git/bmad-engine", "/Users/tony/git/bmad-engine/*"],
  "sessionID": "session_xyz",
  "messageID": "message_def",
  "callID": "call_456",
  "title": "Access file outside working directory: /Users/tony/git/bmad-engine/README.md",
  "metadata": {
    "filepath": "/Users/tony/git/bmad-engine/README.md",
    "parentDir": "/Users/tony/git/bmad-engine"
  },
  "time": {
    "created": 1733629200000
  }
}
```

### REST API: Respond to Permission

**Endpoint**: `POST /session/{sessionID}/permissions/{permissionID}`

**Request Body**:
```json
{
  "response": "once" | "always" | "reject"
}
```

**Response Types**:
- `"once"` - Allow this specific request only
- `"always"` - Allow this and all future matching requests
- `"reject"` - Deny this request

**Example**:
```bash
curl -X POST http://localhost:4096/session/session_xyz/permissions/permission_abc123 \
  -H "Content-Type: application/json" \
  -d '{"response": "once"}'
```

## Proposed Implementation

### Phase 1: Basic Permission Dialog

**Changes to EGUI**:

1. **Subscribe to Permission Events** (`src/client/events.rs`):
   ```rust
   enum UiMsg {
       // ... existing variants
       PermissionRequest(PermissionInfo),
   }
   
   #[derive(Clone, Debug, Deserialize)]
   struct PermissionInfo {
       id: String,
       #[serde(rename = "type")]
       perm_type: String,
       pattern: Option<Vec<String>>,
       session_id: String,
       message_id: String,
       call_id: Option<String>,
       title: String,
       metadata: serde_json::Value,
       time: PermissionTime,
   }
   
   #[derive(Clone, Debug, Deserialize)]
   struct PermissionTime {
       created: u64,
   }
   ```

2. **Parse SSE `permission.updated` Events**:
   ```rust
   // In SSE event handler
   "permission.updated" => {
       if let Ok(info) = serde_json::from_value::<PermissionInfo>(data) {
           let _ = tx.send(UiMsg::PermissionRequest(info));
       }
   }
   ```

3. **Store Pending Permissions** (`src/app.rs`):
   ```rust
   pub struct OpenCodeApp {
       // ... existing fields
       pending_permissions: Vec<PermissionInfo>,
   }
   ```

4. **Display Permission Dialog**:
   ```rust
   fn render_permission_dialog(&mut self, ctx: &egui::Context) {
       if let Some(perm) = self.pending_permissions.first() {
           egui::Window::new("Permission Required")
               .collapsible(false)
               .resizable(false)
               .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
               .show(ctx, |ui| {
                   ui.heading("Agent Permission Request");
                   ui.separator();
                   
                   ui.label(&perm.title);
                   ui.add_space(8.0);
                   
                   // Show metadata details
                   ui.collapsing("Details", |ui| {
                       ui.label(format!("Type: {}", perm.perm_type));
                       if let Some(pattern) = &perm.pattern {
                           ui.label(format!("Pattern: {}", pattern.join(", ")));
                       }
                       ui.label(format!("Metadata: {}", perm.metadata));
                   });
                   
                   ui.add_space(16.0);
                   ui.horizontal(|ui| {
                       if ui.button("❌ Reject").clicked() {
                           self.respond_permission(&perm.session_id, &perm.id, "reject");
                       }
                       if ui.button("✅ Allow Once").clicked() {
                           self.respond_permission(&perm.session_id, &perm.id, "once");
                       }
                       if ui.button("✅ Always Allow").clicked() {
                           self.respond_permission(&perm.session_id, &perm.id, "always");
                       }
                   });
               });
       }
   }
   ```

5. **Send Permission Response** (`src/app.rs`):
   ```rust
   fn respond_permission(&mut self, session_id: &str, perm_id: &str, response: &str) {
       if let Some(client) = &self.client {
           let c = client.clone();
           let sid = session_id.to_string();
           let pid = perm_id.to_string();
           let resp = response.to_string();
           
           if let Some(rt) = &self.runtime {
               rt.spawn(async move {
                   let url = format!("{}/session/{}/permissions/{}", c.base_url, sid, pid);
                   let body = serde_json::json!({"response": resp});
                   let _ = c.http_client.post(&url)
                       .json(&body)
                       .send()
                       .await;
               });
           }
       }
       
       // Remove from pending list
       self.pending_permissions.remove(0);
   }
   ```

6. **Integrate into Main Render Loop** (`src/app.rs`):
   ```rust
   fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
       self.poll_ui_msgs(ctx);
       
       // Render permission dialog on top of everything
       self.render_permission_dialog(ctx);
       
       // ... rest of UI rendering
   }
   ```

### Phase 2: Enhancements (Optional)

**Auto-dismiss on tool completion**:
- If tool completes/errors before user responds, remove permission from queue

**Permission history**:
- Show list of past permissions in settings
- Allow revoking "always" permissions

**Permission preview**:
- For file access: Show file preview
- For bash: Show command breakdown
- For web: Show URL safety info

**Timeout warning**:
- After 60s, show "Request pending for X minutes" warning

## Testing

### Manual Test Case

1. **Setup**: Start EGUI with default permissions (`"ask"`)
2. **Trigger**: Send message: `"Read /tmp/test.txt"`
3. **Expected**: Permission dialog appears with:
   - Title: "Access file outside working directory: /tmp/test.txt"
   - Three buttons: Reject, Allow Once, Always Allow
4. **Click "Allow Once"**
5. **Expected**: Dialog closes, tool executes, file content appears

### Edge Cases

- **Multiple pending permissions**: Queue them, show one at a time
- **Permission for deleted session**: Ignore event
- **Duplicate permission IDs**: Shouldn't happen, but dedupe by ID
- **Network failure on response**: Show error, allow retry

## Alternative: Configuration Bypass

Users who don't want permission dialogs can configure in `~/.opencode/config.json`:

```json
{
  "permission": {
    "external_directory": "allow",
    "bash": "allow",
    "webfetch": "allow",
    "websearch": "allow"
  }
}
```

This bypasses all permission checks and auto-approves. Useful for trusted environments.

## Files to Modify

**EGUI Client**:
- `clients/egui/src/app.rs` - Add permission state, dialog rendering, response logic
- `clients/egui/src/client/events.rs` - Parse `permission.updated` SSE events
- `clients/egui/src/client/api.rs` - Add `respond_permission()` method (if needed)

**Lines of Code**: ~200-300 LOC

**Risk Level**: Low (purely additive, no breaking changes)

## Priority

**MEDIUM-HIGH** - Common user pain point:
- Blocks legitimate workflows (reading external files)
- Confusing UX (appears as hang, not permission wait)
- Simple implementation (mostly UI work)
- Enables security-conscious users to use `"ask"` mode

## Related Issues

- Permission requests currently only visible in TUI client
- Desktop client likely has this implemented already
- No timeout mechanism exists server-side (infinite wait)
