# Session Restoration for OpenCode EGUI

Status: **Planning** (2025-12-08)

## Goals
- Restore conversation tabs from saved sessions on application startup
- Delete sessions from server storage when user closes a tab
- Provide seamless continuation of conversations across app restarts
- Maintain session-per-tab architecture

## Non-Goals
- Session import/export across projects
- Session search/filtering (can be added later)
- Session thumbnails/previews
- Cross-device session sync (server is local only)

## Current Behavior

**On Startup:**
- EGUI client creates one empty tab with no session
- No previous conversations are loaded

**On Tab Close:**
- Tab removed from UI
- Session remains in server storage (orphaned)

## Proposed Behavior

**On Startup:**
1. Fetch list of sessions from server (`GET /session`)
2. For each session:
   - Fetch message history (`GET /session/{id}/message`)
   - Create tab with session ID, title, and messages
   - Restore to last scroll position (bottom)
3. If no sessions exist, create one new empty tab
4. Select first tab as active

**On Tab Close (X button):**
1. Show confirmation if tab has messages: "Delete this conversation?"
2. If confirmed or empty, call `DELETE /session/{id}`
3. Remove tab from UI
4. If last tab closed, create new empty tab
5. Select adjacent tab (or new tab) as active

**New Tab (+button):**
- Call `POST /session` to create new session
- Add new tab to UI
- Switch to new tab

## Architecture Overview

### Data Flow

```
Startup:
EGUI Client                    OpenCode Server               Storage
     |                              |                           |
     |-- GET /session ------------->|                           |
     |                              |-- list sessions --------->|
     |                              |<-- session list ----------|
     |<-- [session IDs/titles] -----|                           |
     |                              |                           |
     |-- GET /session/{id}/message->|                           |
     |                              |-- read messages --------->|
     |                              |<-- message list ----------|
     |<-- [messages] ---------------|                           |
     |                              |                           |
     [Create tab with history]      |                           |
     [Repeat for each session]      |                           |

Tab Close:
EGUI Client                    OpenCode Server               Storage
     |                              |                           |
     [User clicks X on tab]         |                           |
     [Show confirmation dialog]     |                           |
     |-- DELETE /session/{id} ----->|                           |
     |                              |-- remove session -------->|
     |                              |-- remove messages ------->|
     |                              |-- remove parts ---------->|
     |                              |<-- deleted ---------------|
     |<-- success ------------------|                           |
     [Remove tab from UI]           |                           |
```

### Project Scoping

Sessions are automatically scoped to the current **project**:
- Project = Git repository (identified by root commit hash)
- Non-git directories fall back to "global" project
- Server determines project from working directory
- `GET /session` only returns sessions for current project
- No need for EGUI to handle project filtering

Example:
- `/Users/tony/git/opencode` → Project ID: `abc123...` (git root commit)
- `/Users/tony/git/other-repo` → Project ID: `def456...` (different project)
- `/Users/tony/random-folder` → Project ID: `"global"` (no git)

## API Endpoints

### GET /session
Returns list of sessions for current project.

**Request:**
```
GET /session?directory=/Users/tony/git/opencode/clients/egui
```

**Response:**
```json
[
  {
    "id": "session_abc123",
    "title": "New session - 2025-12-08T02:04:28",
    "projectID": "a1b2c3d4e5f6...",
    "directory": "/Users/tony/git/opencode/clients/egui",
    "time": {
      "created": 1733627068736,
      "updated": 1733627428736
    }
  }
]
```

### GET /session/{id}/message
Returns message history for a session.

**Request:**
```
GET /session/{id}/message?directory=/Users/tony/git/opencode/clients/egui
```

**Response:**
```json
[
  {
    "info": {
      "id": "message_001",
      "sessionID": "session_abc123",
      "role": "user",
      "time": { "created": 1733627100000 }
    },
    "parts": [
      {
        "id": "part_001",
        "messageID": "message_001",
        "type": "text",
        "text": "What is the emoji code for a waste paper basket?"
      }
    ]
  },
  {
    "info": {
      "id": "message_002",
      "sessionID": "session_abc123",
      "role": "assistant",
      "time": { "created": 1733627105000 }
    },
    "parts": [
      {
        "id": "part_002",
        "messageID": "message_002",
        "type": "text",
        "text": "🗑 (Unicode: U+1F5D1 U+FE0F)"
      }
    ]
  }
]
```

### DELETE /session/{id}
Deletes a session and all its messages/parts.

**Request:**
```
DELETE /session/{id}?directory=/Users/tony/git/opencode/clients/egui
```

**Response:**
```json
true
```

### POST /session
Creates a new session (already implemented, used for new tabs).

**Request:**
```json
{
  "directory": "/Users/tony/git/opencode/clients/egui"
}
```

**Response:**
```json
{
  "id": "session_xyz789",
  "title": "New session - 2025-12-08T02:24:51",
  "projectID": "a1b2c3d4e5f6...",
  "directory": "/Users/tony/git/opencode/clients/egui",
  "time": {
    "created": 1733627691000,
    "updated": 1733627691000
  }
}
```

## Implementation Plan

### Phase 1: Session Listing on Startup ⏳ TODO
1. Add session restoration to `OpenCodeApp::new()`:
   - After server connection, fetch sessions via `GET /session`
   - Parse session list from JSON response
2. Add session list parsing:
   - Create `SessionInfo` struct matching server response
   - Deserialize session array
3. Create tabs from session list:
   - For each session, create `Tab` with:
     - `session_id: Some(session.id)`
     - `title: session.title`
     - `directory: Some(session.directory)`
     - Empty `messages` (will be populated next phase)
4. Handle empty session list:
   - If no sessions, create one new tab (current behavior)
   - Call `POST /session` to create initial session
5. Set first tab as active

### Phase 2: Message History Restoration ⏳ TODO
1. Add message fetching after session list:
   - For each session, call `GET /session/{id}/message`
   - Parse message history response
2. Add message parsing:
   - Create `MessageWithParts` struct matching server format
   - Extract message info and parts
3. Convert server messages to DisplayMessage:
   - Map `role` to DisplayMessage.role
   - Combine text parts into DisplayMessage.text_parts
   - Extract tool calls from parts (if present)
   - Generate or use message ID from server
4. Populate tab messages:
   - Append DisplayMessage objects to tab.messages
   - Maintain chronological order (server returns reversed)
5. Handle large histories:
   - Optional: Limit to last N messages (e.g., 100)
   - Optional: Add "Load More" button for older messages

### Phase 3: Tab Close with Session Deletion ⏳ TODO
1. Modify tab close handler:
   - Intercept X button click on tab
   - Check if tab has messages (length > 0)
2. Add confirmation dialog:
   - If tab has messages, show modal: "Delete this conversation?"
   - Buttons: "Delete" (primary), "Cancel" (secondary)
   - If empty tab, skip confirmation
3. Call DELETE endpoint:
   - On confirmation, send `DELETE /session/{id}`
   - Wait for response before removing tab
4. Handle deletion errors:
   - Show error toast if deletion fails
   - Keep tab open on error
   - Allow retry
5. Remove tab from UI:
   - Remove from `self.tabs` vector
   - Adjust `self.active` index if needed
6. Handle last tab closed:
   - If `tabs.is_empty()`, create new tab
   - Call `POST /session` for new session

### Phase 4: UI Polish & Edge Cases ⏳ TODO
1. Add loading indicators:
   - Show spinner during session/message fetching
   - "Loading conversations..." message
2. Handle startup errors:
   - Network error fetching sessions → show error, create new tab
   - Invalid session data → skip broken session, continue with others
   - Server not running → show connection error (already handled)
3. Improve tab titles:
   - Show first message preview if available (e.g., "What is the emoji...")
   - Show message count (e.g., "Conversation (5 messages)")
   - Preserve server-provided title if preferred
4. Add session age indicators:
   - Show "Today", "Yesterday", "X days ago" in tab tooltip
   - Use `session.time.updated` for sorting/display
5. Handle rapid tab closing:
   - Prevent double-deletion (disable X button while deleting)
   - Queue multiple deletions if needed
6. Tab order:
   - Optional: Sort tabs by most recently updated
   - Optional: Pin tabs feature
7. Empty state handling:
   - When all tabs deleted, smoothly transition to new tab
   - Don't flash empty UI

## Configuration

### Client Config (config.toml)

```toml
[sessions]
# Maximum number of sessions to restore on startup
# Prevents slowdown with hundreds of old sessions
max_restore = 50

# Whether to show confirmation when closing tabs with messages
confirm_close = true

# Maximum messages to load per session on startup
# Older messages can be loaded on-demand later
max_messages_per_session = 100

# Sort order for restored tabs
# Options: "recent" (most recently updated first), "created" (oldest first)
tab_sort = "recent"
```

## Data Structures

### Rust Structs (app.rs)

```rust
#[derive(Debug, Clone, Deserialize)]
struct SessionInfo {
    id: String,
    title: String,
    #[serde(rename = "projectID")]
    project_id: String,
    directory: Option<String>,
    time: SessionTime,
}

#[derive(Debug, Clone, Deserialize)]
struct SessionTime {
    created: u64,  // Unix timestamp in milliseconds
    updated: u64,
}

#[derive(Debug, Clone, Deserialize)]
struct MessageWithParts {
    info: MessageInfo,
    parts: Vec<MessagePart>,
}

#[derive(Debug, Clone, Deserialize)]
struct MessageInfo {
    id: String,
    #[serde(rename = "sessionID")]
    session_id: String,
    role: String,  // "user", "assistant", "system"
    time: MessageTime,
}

#[derive(Debug, Clone, Deserialize)]
struct MessageTime {
    created: u64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type")]
enum MessagePart {
    #[serde(rename = "text")]
    Text {
        id: String,
        #[serde(rename = "messageID")]
        message_id: String,
        text: String,
    },
    #[serde(rename = "tool")]
    Tool {
        id: String,
        #[serde(rename = "messageID")]
        message_id: String,
        tool: String,
        state: ToolState,
    },
    // Other part types...
}

#[derive(Debug, Clone, Deserialize)]
struct ToolState {
    status: String,  // "pending", "running", "completed", "error"
}
```

### Tab Struct Changes

```rust
#[derive(Default, Clone)]
struct Tab {
    title: String,
    session_id: Option<String>,  // Already exists
    directory: Option<String>,   // Already exists
    messages: Vec<DisplayMessage>,
    input: String,
    
    // New fields for session restoration
    restored: bool,  // True if messages loaded from server
    has_unsaved_input: bool,  // True if input field has text (warn on close)
}
```

## User Experience

### Startup Flow
1. User launches EGUI app
2. Shows "Connecting to server..." (existing)
3. Shows "Loading conversations..." with spinner
4. Tabs appear one by one (or all at once)
5. First tab selected and ready for input

### Closing a Tab
1. User clicks X button on tab
2. If tab has messages:
   - Modal appears: "Delete this conversation?"
   - User confirms or cancels
3. If empty or confirmed:
   - Tab animates out (fade/slide)
   - Adjacent tab selected
4. If last tab, new empty tab appears smoothly

### Error Handling
- **Server unreachable:** Show error, allow retry, create local-only tab as fallback
- **Session fetch fails:** Log error, create new tab, show toast notification
- **Message fetch fails:** Show partial session (title only), allow manual refresh
- **Delete fails:** Keep tab open, show error toast, allow retry

## Testing Checklist

### Session Restoration
- [ ] Empty storage → creates new tab
- [ ] One session with messages → restores single tab
- [ ] Multiple sessions → restores all tabs
- [ ] Session with empty messages → creates tab with no history
- [ ] Very long conversation (100+ messages) → loads without freezing
- [ ] Concurrent sessions across projects → only loads current project

### Tab Closing
- [ ] Close empty tab → deletes immediately, no confirmation
- [ ] Close tab with messages → shows confirmation dialog
- [ ] Cancel confirmation → keeps tab open
- [ ] Confirm deletion → tab closes, session deleted from server
- [ ] Close last tab → creates new tab automatically
- [ ] Close middle tab → adjacent tab selected
- [ ] Network error during delete → shows error, keeps tab

### Edge Cases
- [ ] Start app while server is down → graceful error
- [ ] Server becomes unavailable during restore → partial restore
- [ ] Corrupt session data → skips broken session, continues
- [ ] Delete same session twice (race condition) → handles gracefully
- [ ] Close tab during message streaming → cancels stream, deletes session
- [ ] Switch projects (cd to different git repo) → sessions update correctly

## Future Enhancements (Out of Scope)

- Session search/filter in UI
- Session rename/edit title
- Session archive (hide without deleting)
- Session export to file
- Drag-and-drop tab reordering
- Tab groups/categories
- Session templates
- Auto-save drafts (save input field content)
- Session sharing (multi-user)
- Undo session deletion (recycle bin)
