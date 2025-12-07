# OpenCode EGUI Thin Client — Architecture & Implementation Plan

Status: Draft (2025-12-07)
Owner: Tony (fork)
Location: clients/egui

## Goals
- Provide a native Rust EGUI desktop client as a thin layer over the OpenCode server.
- Auto-detect and attach to a running server; start one if none is found.
- Support multiple sessions (tabbed UI) and streaming assistant output.
- Keep client config (UI + server discovery) local; show a Server Preferences pane.

## Non-Goals
- Replacing/removing the existing 2e/TUI.
- Re-implementing agent logic, tools, or file ops locally.

## Architecture Overview
- Client type: Thin client.
- Transport: HTTP for requests + SSE for event stream.
- Server endpoints used (examples; scoped to change):
  - GET /doc (readiness/health proxy)
  - GET /global/event (SSE stream)
  - Sessions: GET /session, GET /session/:id, POST /session, DELETE /session/:id
  - Config: GET /config, PATCH /config
- Context key: directory passed via header `x-opencode-directory` or query `?directory=` when relevant.

## Server Discovery (Cross-Platform)
- Use crates to hide OS differences; avoid #[cfg] branching in our code.
- Process discovery: sysinfo (list processes, command line matching "bun|node" + "opencode").
- Port discovery: netstat2 (enumerate TCP sockets, match PID, pick LISTEN state → local_port).
- Validation: HTTP GET {base_url}/doc must return 200.
- Startup fallback: If no process found or validation fails → spawn `opencode serve --port 0` and parse the printed line `opencode server listening on http://HOST:PORT`, then validate.
- Result: base_url stored in runtime state and persisted in local config.

Pseudo:
1) try find_opencode_server() → Some(pid, port)? validate(base_url)
2) if valid → attach; else spawn serve, wait until GET /doc succeeds (retry w/backoff)

## Client Modules (proposed layout)
- src/
  - main.rs: eframe bootstrap + startup flow (discover or launch server)
  - app.rs: EGUI app root, panes, and routing
  - orchestrator.rs: mpsc event routing between UI and client API
  - client/
    - api.rs: REST client (reqwest); helpers for sessions/config, directory header
    - events.rs: SSE client wrapper; maps events to typed enums
    - types.rs: minimal DTOs used by the UI
  - ui/
    - tabs.rs: multi-session tabs (adapted from chat-poc pattern)
    - chat.rs: chat transcript with markdown (egui_commonmark)
    - settings.rs: UI preferences (fonts, keybinds, chat density)
    - server_prefs.rs: server URL, directory, auto-start toggle, discovery diagnostics
  - discovery/
    - process.rs: sysinfo + netstat2 integration
    - spawn.rs: start `opencode serve`, capture host:port, readiness loop
  - config.rs: local config file (serde + directories)

## Reference Implementations to Reuse

### 1) bmad-engine (rich chat UI, copy, STT/TTS)
- Path: `/Users/tony/git/bmad-engine`
- Reuse: chat bubble rendering, markdown viewer, copy buttons, font configuration, sidebar/status patterns.
- Defer: STT/TTS audio stack (optional later).

Example (message bubble rendering with copy button):
```rust path=/Users/tony/git/bmad-engine/src/app.rs start=270
fn render_message_bubble(&mut self, ui: &mut egui::Ui, msg: &ChatMessage) {
    let available_width = ui.available_width();
    let bubble_max_width = available_width * 0.75; // Bubbles take max 75% of width
    // ...
    if ui.button("Copy").clicked() {
        ui.ctx().copy_text(msg.content.clone());
    }
}
```

### 2) chat-poc (multi-tab session management)
- Path: `/Users/tony/git/chat-poc/original/chat-poc-core`
- Reuse: tab bar UI and state model for multiple conversations.

Tab switching & add button:
```rust path=/Users/tony/git/chat-poc/original/chat-poc-core/src/main.rs start=269
egui::TopBottomPanel::top("tabs_panel").show(ctx, |ui| {
    ui.horizontal(|ui| {
        for (index, tab) in self.state.tabs.iter().enumerate() {
            if ui.selectable_label(self.state.active_tab == index, &tab.name).clicked() {
                self.state.active_tab = index;
            }
        }
        if ui.button("+").clicked() {
            self.state.add_tab();
        }
    });
});
```

Tab model with add_tab:
```rust path=/Users/tony/git/chat-poc/original/chat-poc-core/src/app_state.rs start=66
impl AppState {
    pub fn new() -> Self {
        AppState { tabs: vec![TabState::new("Tab 1")], active_tab: 0 }
    }
    pub fn active_tab_state(&mut self) -> &mut TabState { &mut self.tabs[self.active_tab] }
    pub fn add_tab(&mut self) {
        let tab_number = self.tabs.len() + 1;
        self.tabs.push(TabState::new(format!("Tab {tab_number}")));
        self.active_tab = self.tabs.len() - 1;
    }
}
```

## Key UX Decisions
- Multi-session tabs: New tab creates a server-side session via POST /session.
- Streaming: Subscribe to /global/event and filter by sessionID.
- Chat density: compact by default; progressive disclosure for tool calls to reduce noise.
- Server Preferences pane: show detected PID/port, a reconnect button, base URL override, and logs for discovery attempts.

## Dependencies (initial)
- eframe, egui, egui_commonmark
- tokio (multi-thread), reqwest (rustls-tls), eventsource-stream or sse-client
- sysinfo, netstat2, thiserror, serde/serde_json, directories

> Critical note on forked dependencies
>
> Both reference implementations rely on Tony's forks of egui, egui_commonmark, and egui-file-dialog. For feature parity (e.g., L/R modifier keys), this client should either pin to these forks or verify upstream support before switching to crates.io.
>
> Evidence:
>
> ```toml path=/Users/tony/git/bmad-engine/Cargo.toml start=9
> egui = { git = "https://github.com/TonyMarkham/egui.git", branch = "feature/left-right-modifiers" }
> eframe = { git = "https://github.com/TonyMarkham/egui.git", branch = "feature/left-right-modifiers" }
> egui_commonmark = { git = "https://github.com/TonyMarkham/egui_commonmark.git", branch = "feature/use-fork-of-egui" }
> ```
>
> ```toml path=/Users/tony/git/bmad-engine/Cargo.toml start=45
> egui-file-dialog = { git = "https://github.com/TonyMarkham/egui-file-dialog.git", branch = "feature/use-egui-fork" }
> ```
>
> Plan: Start with upstream crates if possible; if parity gaps appear (e.g., L/R modifiers), switch to these forks in clients/egui/Cargo.toml and track upstream issues/PRs to migrate back later.

## Config Strategy
- Local file (e.g., $XDG_CONFIG_HOME/opencode-egui/config.json):
  - server: { last_base_url, auto_start: true, directory_override: null }
  - ui: { fonts, keybinds, chat_density }
- Server-side config remains the source of truth for agent/model/workspace settings (queried via /config).

## Milestones
- M0 — Bootstrap
  - Scaffold crate in clients/egui (Cargo.toml, main.rs)
  - Render blank EGUI window
- M1 — Server Discovery
  - Implement sysinfo + netstat2 detection
  - Implement spawn + readiness validation
  - Persist last discovered base_url in local config
- M2 — Events + Sessions
  - SSE subscription with backoff and UI diagnostics
  - Sessions list/create/delete; open session → new tab
  - Stream assistant output to active tab
- M3 — Server Preferences + Settings
  - Dedicated pane for server controls + diagnostics
  - Local UI preferences with live application
- M4 — Polishing
  - Chat markdown rendering, copy buttons, minimal tool-call summaries
  - Error surfaces and reconnect flows

## Risks & Mitigations
- Server API drift → keep the client small and resilient; feature-gate advanced UI until endpoints stabilize.
- Discovery false positives → require validation via GET /doc; allow manual URL override.
- SSE disconnects → retry with backoff; show non-blocking banner.

## Next Steps (actionable)
1) Initialize clients/egui crate and add dependencies.
2) Implement discovery::process and discovery::spawn.
3) Add client::api (GET /doc, sessions CRUD) and client::events (SSE).
4) Build tabbed UI using chat-poc pattern; wire to sessions + streaming.
5) Add Server Preferences pane and local config persistence.
