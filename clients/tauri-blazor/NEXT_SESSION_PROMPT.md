# Next Session: Shared Rust Core & Project Scaffold

## Quick Context

**What We Completed (Session 0 - 2026-01-02):**

- ✅ Analyzed ADR-0001 scope and requirements
- ✅ Created comprehensive 6-session implementation plan
- ✅ Identified natural session boundaries and dependencies
- ✅ Documented project structure in `clients/tauri-blazor/README.md`

**Current State:**

- ADR-0001 is approved (status: Proposed)
- egui client exists with server discovery logic in `clients/egui/src/discovery/`
- No shared Rust crate exists yet
- `clients/tauri-blazor/` directory created with README.md documenting structure
- **BUT:** We need to extract shared code and create project structure

**CRITICAL: Read `clients/tauri-blazor/README.md` FIRST to understand the Cognexus-inspired project structure before starting implementation.**

---

## Your Mission: Session 1

**Create the foundation:** Extract shared Rust client code and scaffold the Tauri-Blazor project structure.

### Step 1: Create Shared Client Core Crate

**Goal:** Extract server discovery and API client logic into a reusable workspace crate

**Tasks:**

1. Create `crates/opencode-client-core/` directory structure
2. Create `Cargo.toml` with proper workspace configuration
3. Create `src/lib.rs` with module exports
4. Create `src/discovery/` module structure (mod.rs, process.rs, spawn.rs)
5. Copy server discovery code from `clients/egui/src/discovery/` to shared crate
6. Extract common types and utilities needed by both clients
7. Update `clients/egui/Cargo.toml` to depend on shared crate
8. Update `clients/egui/src/main.rs` and related files to use shared crate
9. Test that egui client still builds and runs correctly

**Technical Details:**

**Crate structure:**

```
crates/opencode-client-core/
├── Cargo.toml
└── src/
    ├── lib.rs
    ├── discovery/
    │   ├── mod.rs
    │   ├── process.rs   # Port scanning, process detection
    │   └── spawn.rs     # Server spawning logic
    ├── types/
    │   ├── mod.rs
    │   └── server.rs    # ServerInfo struct
    └── error/
        ├── mod.rs
        └── discovery.rs # Error types
```

**Dependencies to include:**

- tokio (rt-multi-thread, macros, process)
- serde (with derive)
- serde_json
- reqwest (json, rustls-tls)
- sysinfo
- netstat2
- thiserror

**Key patterns:**

- Use `#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]` on ServerInfo
- Export public API from `lib.rs`: `pub mod discovery; pub mod types; pub mod error;`
- Keep the same function signatures as egui currently uses

---

### Step 2: Create Tauri-Blazor Project Scaffold

**Goal:** Set up directory structure and minimal Tauri configuration

**Tasks:**

1. Create `clients/tauri-blazor/` directory
2. Create `clients/tauri-blazor/src-tauri/` directory structure
3. Create `clients/tauri-blazor/src-tauri/Cargo.toml` with Tauri dependencies
4. Create `clients/tauri-blazor/src-tauri/build.rs` for Tauri build script
5. Create `clients/tauri-blazor/src-tauri/tauri.conf.json` configuration
6. Create `clients/tauri-blazor/src-tauri/src/main.rs` with minimal Tauri app
7. Create `clients/tauri-blazor/src-tauri/src/commands/mod.rs` module structure
8. Create `clients/tauri-blazor/src-tauri/src/state.rs` for app state
9. Create `clients/tauri-blazor/README.md` with setup instructions
10. Create `.gitignore` for Tauri and Blazor artifacts

**Technical Details:**

**Tauri Configuration (`tauri.conf.json`):**

```json
{
  "$schema": "https://schema.tauri.app/config/2",
  "productName": "OpenCode",
  "version": "0.0.1",
  "identifier": "com.opencode.tauri-blazor",
  "build": {
    "frontendDist": "./frontend/wwwroot"
  },
  "app": {
    "withGlobalTauri": true,
    "windows": [
      {
        "title": "OpenCode",
        "width": 1200,
        "height": 800
      }
    ]
  }
}
```

**Minimal main.rs:**

```rust
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod state;

use tauri::Manager;
use state::AppState;

fn main() {
    tauri::Builder::default()
        .setup(|app| {
            let state = AppState::new();
            app.manage(state);
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

**Dependencies in Cargo.toml:**

- tauri (version 2, features: devtools, macos-private-api)
- serde (with derive)
- tokio (full features)
- opencode-client-core (workspace path)

---

### Step 3: Verify Everything Works

**Goal:** Ensure both egui and tauri scaffold build successfully

**Tasks:**

1. Build egui client: `cd clients/egui && cargo build`
2. Run egui client: `cd clients/egui && cargo run`
3. Test server discovery still works in egui
4. Build Tauri project: `cd clients/tauri-blazor/src-tauri && cargo build`
5. Fix any compilation errors
6. Document any issues or decisions made

---

## Success Criteria for Session 1

- [ ] `crates/opencode-client-core` exists and compiles
- [ ] egui client builds and runs with shared crate (no regression)
- [ ] Server discovery still works in egui client
- [ ] `clients/tauri-blazor/src-tauri` compiles successfully
- [ ] No custom JavaScript files created (should be NONE at this stage)
- [ ] All code follows OpenCode style guide (no unnecessary destructuring, prefer single-word vars)

---

## Key Files to Reference

**Existing (Read these first):**

- `clients/egui/src/discovery/mod.rs` - Server discovery entry point
- `clients/egui/src/discovery/process.rs` - Port scanning, process detection
- `clients/egui/src/discovery/spawn.rs` - Server spawning logic
- `clients/egui/Cargo.toml` - Dependencies to copy
- `clients/egui/src/error/discovery.rs` - Error types to extract

**To Create:**

- `crates/opencode-client-core/Cargo.toml` - Shared crate manifest
- `crates/opencode-client-core/src/lib.rs` - Public API exports
- `crates/opencode-client-core/src/discovery/mod.rs` - Discovery module
- `clients/tauri-blazor/src-tauri/Cargo.toml` - Tauri manifest
- `clients/tauri-blazor/src-tauri/src/main.rs` - Tauri entry point
- `clients/tauri-blazor/src-tauri/tauri.conf.json` - Tauri config
- `clients/tauri-blazor/README.md` - Setup instructions

---

## Important Reminders

1. **Production-grade only** - No placeholders, no TODOs without implementation path
2. **Read existing patterns first** - Study egui code before extracting
3. **Test egui after changes** - Don't break the working client
4. **No custom JavaScript** - This session shouldn't need any, but critical for later
5. **Follow style guide** - Avoid `else`, avoid `try/catch`, prefer single-word vars
6. **Edition 2024** - Use latest Rust edition (matching egui)

---

## Workspace Integration

**Update root `Cargo.toml` workspace members:**

```toml
[workspace]
members = [
    "crates/opencode-client-core",
    "clients/egui",
    # ... other members
]
```

**Use workspace dependencies pattern:**

```toml
# In opencode-client-core/Cargo.toml
[dependencies]
tokio = { version = "1.43", features = ["rt-multi-thread", "macros", "process"] }

# In clients/egui/Cargo.toml
[dependencies]
opencode-client-core = { path = "../../crates/opencode-client-core" }
```

---

## Expected Challenges

1. **Module visibility:** Ensure functions are `pub` in shared crate
2. **Error type unification:** May need to create common error enum
3. **Async runtime:** Ensure tokio runtime is properly shared
4. **Path dependencies:** Relative paths need to be correct from both clients
5. **Feature flags:** May need to make some features optional

---

## What NOT to Do

- ❌ Don't create Blazor project yet (that's Session 3)
- ❌ Don't implement Tauri commands yet (that's Session 2)
- ❌ Don't create any `.js` files
- ❌ Don't modify server code (only client-side)
- ❌ Don't change egui's behavior (only its dependencies)

---

## Definition of Done

When you can run:

1. `cd clients/egui && cargo run` → egui client launches and discovers server
2. `cd clients/tauri-blazor/src-tauri && cargo build` → Tauri backend compiles
3. `cargo test -p opencode-client-core` → Shared crate tests pass (if any added)

---

**Start with:** "I'll extract the server discovery code from egui into a shared crate, then create the Tauri project scaffold."

---

**Estimated Time:** 4-6 hours  
**Token Budget:** ~80K tokens  
**Next Session:** Session 2 - Tauri Backend & Server Commands
