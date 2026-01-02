# Next Session: Tauri Backend & Server Commands

## Quick Context

**What We Completed (Session 1 - 2026-01-02):**

- ✅ Created workspace at `clients/tauri-blazor/` with proper Cargo.toml
- ✅ Built production-grade `backend/client-core/` crate with discovery + spawn logic
- ✅ Built `common/` crate for shared ErrorLocation utilities
- ✅ Implemented complete error handling (CoreError, DiscoveryError, SpawnError)
- ✅ Implemented discovery module (discover, stop_pid, check_health)
- ✅ Implemented spawn module (spawn_and_wait with exponential backoff)
- ✅ Zero magic numbers, DRY helpers, full rustdoc, clippy clean

**Current State:**

- `backend/client-core` compiles and passes `cargo clippy -p client-core -- -D warnings` ✅
- All discovery and spawn logic is production-ready
- Workspace structure is set up correctly
- **BUT:** No Tauri backend yet - we need to scaffold `apps/desktop/opencode/` and wire up commands

---

## Your Mission: Session 2 - Tauri Backend Scaffold

Build the Tauri backend that exposes `client-core` functionality via Tauri commands, following the zero-custom-JavaScript policy.

### Step 1: Scaffold Tauri Project

**Goal:** Create the `apps/desktop/opencode/` directory with proper Tauri configuration

**Tasks:**

1. Initialize Tauri 2 project in `clients/tauri-blazor/apps/desktop/opencode/`
   - Use Tauri CLI: `cargo tauri init` (or manually create structure)
   - Target Tauri 2.9.5+ (match Cognexus proven version)
   - Configure for Blazor frontend (HTML/WASM loading)

2. Create `apps/desktop/opencode/Cargo.toml` with dependencies:

   ```toml
   [package]
   name = "opencode"
   version = "0.1.0"
   edition = "2021"

   [dependencies]
   tauri = { version = "2", features = ["protocol-asset"] }
   tokio = { version = "1", features = ["full"] }
   serde = { version = "1", features = ["derive"] }
   serde_json = "1"
   reqwest = { version = "0.12", features = ["json"] }

   # Workspace dependencies
   client-core = { path = "../../../backend/client-core" }
   common = { path = "../../../common" }

   [build-dependencies]
   tauri-build = { version = "2" }
   ```

3. Create `apps/desktop/opencode/tauri.conf.json` with proper configuration:
   - Set app name, identifier (e.g., `com.opencode.blazor`)
   - Configure window properties (min size, title, etc.)
   - Set up frontend dev server URL and build path
   - Configure bundle/build settings
   - **Important:** Enable `protocol-asset` for loading Blazor WASM

4. Create `apps/desktop/opencode/build.rs` with Tauri build script:

   ```rust
   fn main() {
       tauri_build::build()
   }
   ```

5. Update workspace `Cargo.toml` to add new member: `"apps/desktop/opencode"`

6. Verify scaffold compiles: `cargo build -p opencode`

**Technical Details:**

- Follow Tauri 2.x conventions (NOT Tauri 1.x)
- Use `protocol-asset` for serving Blazor static files
- Configure CSP (Content Security Policy) to allow WASM execution
- Set minimum window size appropriate for chat UI (e.g., 800×600)

---

### Step 2: Implement Tauri State Management

**Goal:** Create shared application state for managing server connection

**Tasks:**

1. Create `apps/desktop/opencode/src/state.rs`:

   ```rust
   use std::sync::{Arc, Mutex};

   /// Application state shared across all Tauri commands
   #[derive(Debug, Default)]
   pub struct AppState {
       /// Currently connected server info (if any)
       pub server: Arc<Mutex<Option<ServerInfo>>>,
   }

   #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
   pub struct ServerInfo {
       pub pid: u32,
       pub host: String,
       pub port: u16,
   }
   ```

2. Update `apps/desktop/opencode/src/main.rs` to initialize state:

   ```rust
   #[cfg_attr(mobile, tauri::mobile_entry_point)]
   pub fn run() {
       tauri::Builder::default()
           .manage(AppState::default())
           .invoke_handler(tauri::generate_handler![/* commands here */])
           .run(tauri::generate_context!())
           .expect("error while running tauri application");
   }
   ```

3. Verify state is accessible in commands (we'll test in Step 3)

**Technical Details:**

- Use `Arc<Mutex<T>>` for thread-safe state (Tauri commands run on threadpool)
- `ServerInfo` must derive `Serialize/Deserialize` for Tauri IPC
- State is managed by Tauri's DI system via `.manage()`

---

### Step 3: Implement Server Discovery Commands

**Goal:** Wire up `client-core` discovery logic to Tauri commands

**Tasks:**

1. Create `apps/desktop/opencode/src/commands/mod.rs`:

   ```rust
   pub mod server;
   ```

2. Create `apps/desktop/opencode/src/commands/server.rs`:

   ```rust
   use tauri::State;
   use crate::state::{AppState, ServerInfo};
   use client_core::discovery;

   /// Discovers a running OpenCode server on localhost
   #[tauri::command]
   pub async fn discover_server(
       state: State<'_, AppState>,
   ) -> Result<Option<ServerInfo>, String> {
       // Call client_core::discovery::discover()
       // Parse result and update state
       // Return ServerInfo or None
   }

   /// Spawns a new OpenCode server and waits for health check
   #[tauri::command]
   pub async fn spawn_server(
       state: State<'_, AppState>,
   ) -> Result<ServerInfo, String> {
       // Call client_core::spawn::spawn_and_wait()
       // Update state with new server info
       // Return ServerInfo
   }

   /// Checks if the server is healthy
   #[tauri::command]
   pub async fn check_health(
       state: State<'_, AppState>,
   ) -> Result<bool, String> {
       // Get server info from state
       // Call client_core::discovery::check_health()
       // Return true/false
   }

   /// Stops the currently connected server
   #[tauri::command]
   pub async fn stop_server(
       state: State<'_, AppState>,
   ) -> Result<(), String> {
       // Get server PID from state
       // Call client_core::discovery::stop_pid()
       // Clear state
   }
   ```

3. Wire commands into `main.rs`:

   ```rust
   use commands::server::{discover_server, spawn_server, check_health, stop_server};

   .invoke_handler(tauri::generate_handler![
       discover_server,
       spawn_server,
       check_health,
       stop_server,
   ])
   ```

4. Implement proper error conversion:
   - Convert `client_core::error::CoreError` to `String` for Tauri IPC
   - Include error location information in error messages
   - Log errors with context

**Technical Details:**

- All Tauri commands must be `async` and return `Result<T, String>`
- Use `State<'_, AppState>` to access shared state
- Error strings should be informative (include error location)
- Commands run on Tokio threadpool (safe to call async functions)

**Pattern to follow:**

```rust
#[tauri::command]
pub async fn example_command(
    state: State<'_, AppState>,
) -> Result<ReturnType, String> {
    match client_core::some_function() {
        Ok(result) => {
            // Update state if needed
            Ok(result)
        }
        Err(e) => {
            // Convert error with location
            Err(format!("Failed to do thing: {}", e))
        }
    }
}
```

---

### Step 4: Create Minimal HTML Test Frontend

**Goal:** Test Tauri commands from browser console before building Blazor UI

**Tasks:**

1. Create `apps/desktop/opencode/frontend/index.html`:

   ```html
   <!DOCTYPE html>
   <html>
     <head>
       <meta charset="UTF-8" />
       <title>OpenCode Blazor (Test)</title>
     </head>
     <body>
       <h1>OpenCode Tauri Backend Test</h1>
       <p>Open browser console and test commands:</p>
       <pre>
   // Discover server
   await window.__TAURI__.invoke('discover_server')
   
   // Spawn server
   await window.__TAURI__.invoke('spawn_server')
   
   // Check health
   await window.__TAURI__.invoke('check_health')
   
   // Stop server
   await window.__TAURI__.invoke('stop_server')
       </pre>
     </body>
   </html>
   ```

2. Update `tauri.conf.json` to point to this HTML:

   ```json
   {
     "build": {
       "frontendDist": "./frontend"
     }
   }
   ```

3. Test the commands:

   ```bash
   cd clients/tauri-blazor/apps/desktop/opencode
   cargo tauri dev
   ```

   - Open browser console in Tauri window
   - Run `await window.__TAURI__.invoke('discover_server')`
   - Verify it returns server info or null
   - Test spawn, health check, and stop commands

4. Verify full flow:
   - Spawn server → returns PID, host, port
   - Check health → returns true
   - Stop server → succeeds
   - Check health → returns false or error

**Technical Details:**

- This is a TEMPORARY test frontend (will be replaced with Blazor in Session 3)
- `window.__TAURI__.invoke()` is the Tauri IPC mechanism
- Commands return Promises that resolve/reject based on Rust Result
- All testing from browser console (no custom JS files needed)

---

## Success Criteria for Session 2

- [ ] `apps/desktop/opencode/` directory created with proper Tauri 2 structure
- [ ] Workspace Cargo.toml updated with new member
- [ ] `cargo build -p opencode` succeeds
- [ ] `cargo clippy -p opencode -- -D warnings` passes
- [ ] Tauri app launches with test HTML frontend
- [ ] `discover_server` command works from browser console
- [ ] `spawn_server` command spawns server and returns info
- [ ] `check_health` command returns correct health status
- [ ] `stop_server` command stops server gracefully
- [ ] State management works (server info persists between commands)
- [ ] Error messages include location information from ErrorLocation

---

## Key Files to Reference

**Existing (Read these first):**

- `clients/tauri-blazor/backend/client-core/src/lib.rs` - Public API to wire up
- `clients/tauri-blazor/backend/client-core/src/discovery/mod.rs` - Discovery functions
- `clients/tauri-blazor/backend/client-core/src/spawn/mod.rs` - Spawn functions
- `clients/tauri-blazor/backend/client-core/src/error.rs` - Error types to convert
- `clients/tauri-blazor/common/src/lib.rs` - ErrorLocation trait
- `clients/tauri-blazor/Cargo.toml` - Workspace structure
- `clients/tauri-blazor/README.md` - Project structure

**To Create:**

- `clients/tauri-blazor/apps/desktop/opencode/Cargo.toml` - Tauri package
- `clients/tauri-blazor/apps/desktop/opencode/tauri.conf.json` - Tauri config
- `clients/tauri-blazor/apps/desktop/opencode/build.rs` - Build script
- `clients/tauri-blazor/apps/desktop/opencode/src/main.rs` - Entry point
- `clients/tauri-blazor/apps/desktop/opencode/src/state.rs` - State management
- `clients/tauri-blazor/apps/desktop/opencode/src/commands/mod.rs` - Commands module
- `clients/tauri-blazor/apps/desktop/opencode/src/commands/server.rs` - Server commands
- `clients/tauri-blazor/apps/desktop/opencode/frontend/index.html` - Test HTML

**Reference (for Tauri patterns):**

- Look at other Tauri projects in repo if any exist
- Tauri 2 docs for command patterns
- Cognexus example (same Tauri version)

---

## Important Reminders

1. **Production-grade only** - Match the quality of `client-core`
2. **Tauri 2.x** - NOT Tauri 1.x (different API)
3. **Zero custom JavaScript** - Test HTML only uses `window.__TAURI__.invoke()`
4. **Error handling** - Convert CoreError properly, include location info
5. **State management** - Use Arc<Mutex<T>> for thread safety
6. **Clippy clean** - Must pass `-D warnings`
7. **Full rustdoc** - Document all public APIs
8. **Test as you go** - Verify each command works before moving on

---

## Technical Constraints

- **Tauri Version:** 2.9.5+ (match Cognexus)
- **.NET Target (future):** 9.0+ for Blazor WASM
- **Zero Custom JavaScript:** All IPC via C# IJSRuntime only (Blazor in Session 3)
- **Protocol:** Use `protocol-asset` for serving static files
- **Window:** Minimum 800×600, resizable, proper title

---

## Estimated Token Budget

**~100K tokens:**

- Reading context: ~15K tokens (existing code + docs)
- Tauri scaffold: ~20K tokens
- State management: ~10K tokens
- Command implementation: ~30K tokens
- Testing/verification: ~15K tokens
- Documentation: ~10K tokens

---

**Start with:** "Let me read the existing client-core API to understand what we're wiring up, then scaffold the Tauri project structure."
