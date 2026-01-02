# Session Plan: Tauri + Blazor Desktop Client (ADR-0001)

## Goal

Build a new Tauri + Blazor WebAssembly desktop client as an alternative to egui, sharing Rust backend code and following the zero-custom-JavaScript policy.

---

## Session 1: Shared Rust Core & Project Scaffold

### Step 1: Extract Shared Client Core

- ✅ Create shared workspace at `clients/tauri-blazor/`
- ✅ Build production-grade `backend/client-core/` crate from scratch
- ✅ Build production-grade `common/` crate for shared utilities
- ⏭️ Extract from egui (DEFERRED - built fresh code instead)
- ⏭️ Update egui to use shared crate (DEFERRED - egui unchanged)

### Step 2: Create Tauri-Blazor Directory Structure

- ✅ Create `clients/tauri-blazor/` workspace layout
- ✅ Create workspace Cargo.toml with proper dependencies
- ✅ Create README.md documenting structure
- ⏭️ Set up Tauri project skeleton (`apps/desktop/opencode/`) (DEFERRED to Session 2)

**Status:** ✅ Complete

**Actual Tokens:** ~60K

**Deliverables:**

- ✅ Working `backend/client-core` with discovery + spawn logic
- ✅ Working `common` crate with ErrorLocation utilities
- ✅ egui client unchanged (no breaking changes)
- ⏭️ Tauri project structure (deferred to Session 2)

---

## Session 2: Tauri Backend & Server Commands

### Step 1: Implement Tauri State Management

- Create `apps/desktop/opencode/src/state.rs` for shared app state
- Set up state initialization in main.rs

### Step 2: Implement Server Discovery Commands

- Create `apps/desktop/opencode/src/commands/server.rs`
- Implement `discover_server()`, `spawn_server()`, `check_health()`, `stop_server()`
- Wire commands into Tauri builder

### Step 3: Test Tauri Commands

- Build Tauri app with minimal HTML frontend
- Test commands from browser console
- Verify server discovery/spawn works

**Status:** ⏳ Pending

**Estimated Tokens:** ~100K

**Deliverables:**

- ✅ Tauri commands working for server operations
- ✅ Can discover/spawn OpenCode server from Tauri
- ✅ State management functional

---

## Session 3: Blazor Frontend Scaffold & Server Integration

### Step 1: Initialize Blazor WASM Project

- Create `frontend/` directory with .NET project
- Configure OpenCodeBlazor.csproj (Radzen, Markdig packages)
- Set up Program.cs with dependency injection
- Configure publish to `apps/desktop/opencode/frontend/`

### Step 2: Create Server Service Layer

- Implement `IServerService` and `ServerService`
- Use `IJSRuntime` to call Tauri commands (NO custom JS)
- Create ServerInfo C# model matching Rust

### Step 3: Build Basic UI

- Create Home.razor with server status display
- Add discover/spawn buttons with Radzen components
- Test full flow: Blazor → C# → IJSRuntime → Tauri → Rust

**Status:** ⏳ Pending

**Estimated Tokens:** ~120K

**Deliverables:**

- ✅ Blazor WASM project compiles and publishes
- ✅ Tauri loads Blazor UI successfully
- ✅ Server discovery/spawn working from UI

---

## Session 4: Chat UI & Message Streaming

### Step 1: Implement Session Service

- Create `ISessionService` and `SessionService`
- Add Tauri commands for session creation
- Implement session management in Rust backend

### Step 2: Build Chat Components

- Create `MessageList.razor` for conversation history
- Create `InputBox.razor` for message input
- Create `SessionTabs.razor` for multi-session support

### Step 3: Implement Streaming

- Add SSE client in Blazor or use Tauri event system
- Stream message chunks from Claude API
- Update UI reactively as chunks arrive
- Integrate Markdig for markdown rendering

**Status:** ⏳ Pending

**Estimated Tokens:** ~120K

**Deliverables:**

- ✅ Chat UI with message history
- ✅ Streaming responses working
- ✅ Markdown rendered properly

---

## Session 5: Authentication & Settings

### Step 1: OAuth Integration

- Extract auth logic from egui to shared core
- Create `commands/auth.rs` in Tauri
- Implement OAuth flow for Anthropic
- Add API key management

### Step 2: Settings Panel

- Create `Settings.razor` page
- Add model selection UI
- Add provider configuration
- Add theme/UI preferences

### Step 3: Persistence

- Implement settings save/load
- Store auth tokens securely
- Session restoration

**Status:** ⏳ Pending

**Estimated Tokens:** ~100K

**Deliverables:**

- ✅ OAuth authentication working
- ✅ Settings panel functional
- ✅ Preferences persisted across restarts

---

## Session 6: Polish, Testing & Documentation

### Step 1: Cross-Platform Testing

- Test on macOS, Windows, Linux
- Fix platform-specific issues
- Optimize performance

### Step 2: Build System Integration

- Create justfile for build orchestration
- Integrate with monorepo tooling (turbo.json)
- Set up packaging/bundling

### Step 3: Documentation

- Update README with usage instructions
- Document differences from egui client
- Create "Choosing a Desktop Client" guide

**Status:** ⏳ Pending

**Estimated Tokens:** ~80K

**Deliverables:**

- ✅ Client works on all platforms
- ✅ Build process streamlined
- ✅ Documentation complete

---

## Success Criteria

### Session 1

- [x] `backend/client-core` builds successfully with clippy `-D warnings`
- [x] Production-grade error handling with ErrorLocation tracking
- [x] Discovery module complete (discover, stop_pid, check_health)
- [x] Spawn module complete (spawn_and_wait with exponential backoff)
- [x] Zero magic numbers, all DRY, full rustdoc
- [x] egui client unchanged (no breaking changes)
- [ ] Tauri project structure created (deferred to Session 2)

### Session 2

- [ ] Tauri commands for server operations work
- [ ] Can discover running OpenCode server
- [ ] Can spawn new OpenCode server

### Session 3

- [ ] Blazor WASM compiles and loads in Tauri
- [ ] Server discovery UI works
- [ ] No custom JavaScript files created

### Session 4

- [ ] Chat interface functional
- [ ] Messages stream in real-time
- [ ] Markdown renders correctly

### Session 5

- [ ] OAuth login successful
- [ ] Settings persist across restarts
- [ ] Multiple providers supported

### Session 6

- [ ] Builds on all platforms
- [ ] Performance acceptable (<2s startup)
- [ ] Documentation published

---

## Notes & Decisions

### Session 1 (2026-01-02) ✅

**Accomplishments:**

- ✅ Created workspace at `clients/tauri-blazor/` with Cargo.toml
- ✅ Built `backend/client-core/` crate from scratch (NOT extracted from egui)
- ✅ Built `common/` crate for shared ErrorLocation utilities
- ✅ Implemented production-grade error handling:
  - `CoreError`, `DiscoveryError`, `SpawnError` with ErrorLocation tracking
  - All errors use `common` crate for location tracking
- ✅ Implemented discovery module:
  - `discover()` - finds running opencode server via `ps` + regex
  - `stop_pid()` - graceful shutdown with exponential backoff kill verification
  - `check_health()` - HTTP GET with exponential backoff retry
- ✅ Implemented spawn module:
  - `spawn_and_wait()` - spawns server and waits for health check
  - Exponential backoff for health checks
  - Process cleanup on failure paths
  - Stderr capture for debugging
- ✅ Zero magic numbers - all constants named
- ✅ DRY helpers throughout
- ✅ Regex compiled once with OnceLock
- ✅ Full rustdoc on public APIs
- ✅ Clippy clean with `-D warnings`

**Files Created:**

- `clients/tauri-blazor/Cargo.toml` - workspace root
- `clients/tauri-blazor/common/Cargo.toml` - shared utilities
- `clients/tauri-blazor/common/src/lib.rs` - ErrorLocation trait
- `clients/tauri-blazor/backend/client-core/Cargo.toml` - core logic
- `clients/tauri-blazor/backend/client-core/src/lib.rs` - public API
- `clients/tauri-blazor/backend/client-core/src/error.rs` - error types
- `clients/tauri-blazor/backend/client-core/src/discovery/mod.rs` - discovery module
- `clients/tauri-blazor/backend/client-core/src/discovery/process.rs` - process logic
- `clients/tauri-blazor/backend/client-core/src/spawn/mod.rs` - spawn module
- `clients/tauri-blazor/README.md` - project structure docs

**Technical Decisions:**

- **Built fresh instead of extracting from egui** - Allows production-grade code without egui constraints
- **Located code in `clients/tauri-blazor/backend/client-core/`** - Not `crates/` since it's tauri-specific for now
- **Created `common/` crate** - Shared utilities between backend crates
- **ErrorLocation pattern** - Consistent error tracking across all error types
- **OnceLock for regex** - Compile once, reuse across all calls
- **Exponential backoff everywhere** - Robust retry logic for health checks and process cleanup
- **No magic numbers** - All timeouts, delays, retries are named constants

**Deferred to Session 2:**

- Tauri scaffold (`apps/desktop/opencode/`)
- No changes to egui client (remains independent)

**Next Steps:**

- Session 2 will scaffold Tauri backend and wire up commands to `client-core`

---

## Technical Constraints

1. **Zero Custom JavaScript** - All Tauri IPC via C# IJSRuntime only
2. **.NET 9.0+** - Target modern .NET for best Blazor support
3. **Tauri 2.9.5+** - Match Cognexus proven version
4. **Radzen Components** - Use for all UI (no custom DOM manipulation)
5. **Shared Rust Code** - Maximize code reuse between egui and tauri-blazor

---

## Dependencies

- **Session 2** depends on Session 1 (needs shared crate)
- **Session 3** depends on Session 2 (needs Tauri commands)
- **Session 4** depends on Session 3 (needs Blazor scaffold)
- **Session 5** depends on Session 4 (needs chat working)
- **Session 6** depends on Sessions 1-5 (integration/polish)

---

## Risk Mitigation

**Risk:** Breaking egui client while extracting code

- **Mitigation:** Test egui after each extraction, make changes incrementally

**Risk:** JSInterop for Tauri not working as expected

- **Mitigation:** Create minimal test case in Session 2 before building full UI

**Risk:** Blazor WASM bundle size too large

- **Mitigation:** Use trimming options, analyze bundle size early

**Risk:** Streaming performance issues

- **Mitigation:** Benchmark early, consider Tauri events vs SSE

---

## Open Questions

1. ~~Should we use .NET 9 or .NET 10?~~ → **Use .NET 9 (more stable, LTS-adjacent)**
2. ~~justfile vs Bun integration?~~ → **Start with justfile (proven in Cognexus), integrate with Bun later**
3. Audio/STT approach? → **Defer to future session (not in MVP)**
4. Mobile support? → **No, desktop only for now**

---

## Total Estimated Effort

**6 sessions × ~100K average = ~600K tokens total**

**Timeline Estimate:** 6-8 weeks (1 session per week, with buffer for discoveries)

---

**Last Updated:** 2026-01-02
