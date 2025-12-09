# Feature: Per-tab Agent Picker in eGUI

Problem
The desktop eGUI connects to an opencode server that exposes multiple agents (build, plan, etc.). eGUI does not currently surface the server’s agents as a per‑tab choice. Users want a left-hand collapsible panel listing all enabled server agents and the ability to switch the active agent for the current tab.

## Current State

- Server
  - GET /agent returns the full agent catalog (built-ins + configured). The prompt API accepts an optional agent field and defaults to "build" when omitted.
- eGUI (clients/egui)
  - OpenCodeApp now fetches agents on server connect, stores per-tab `selected_agent`, defaults to the first primary agent (fallback "build"), and tracks subagent toggle + pane collapse in memory.
  - Left Agents SidePanel and tab header label show the current agent; prompt send now includes the selected agent.
  - HTTP client (client/api.rs) adds `list_agents` and carries an optional `agent` in the message payload.

## Proposed Changes

- Data model
  - Add AgentInfo (id: name, description?, mode, builtIn?, color?) to match server response fields actually used by eGUI (name, mode, builtIn, color optional).
  - Extend Tab with selected_agent: Option<String>.
- Networking
  - Add OpencodeClient::list_agents() -> Result<Vec<AgentInfo>, ApiError> that GETs /agent.
  - On server connect (UiMsg::ServerConnected branch), fetch agents and cache in app state (Vec<AgentInfo>). Filter display to non-"subagent" by default; expose a toggle in Settings to show all.
- UI (left collapsible pane)
  - Add egui::SidePanel::left("agents_pane") visible only in a session tab. Title: "Agents" with a collapse/expand chevron (persist last state in config).
  - List agents vertically (radio/select style). Show name with a subtle built-in badge for builtIn=true. Optional color swatch using provided color or a derived palette.
  - Clicking an agent sets tab.selected_agent = Some(name). The selected item is highlighted.
  - In the tab header, append the agent name (e.g., "build") next to model display to make the current choice visible even when the pane is collapsed.
- Sending prompts
  - When sending a prompt, include agent: tab.selected_agent.unwrap_or(default_primary) in the request body. If message sending is not yet wired, prepare the field so future send integrates cleanly.
- Defaults & behavior
  - Default the first time a tab is created to the first primary agent returned by the server (fallback to "build").
  - If the server agent list changes during runtime, keep the current selection if still present; otherwise revert to default and show a toast.
- Settings
  - Add a checkbox in Settings → UI to "Show subagents in agent list" (off by default).
  - Persist agents_pane_collapsed and last selected agent per tab (in memory only; no disk persistence needed initially).

## Status

- Implemented client-side agent discovery, selection UI, and agent-aware prompt payloads in `src/app.rs`, `src/client/api.rs`, `src/types/agent.rs`, and `src/types/models.rs`.
- `cargo check` passes; existing unrelated warnings remain; tests still pending.

## Server API Endpoints (used by eGUI)

- List agents: GET `/agent` → returns array of Agent objects. Source: packages/opencode/src/server/server.ts (operationId: app.agents).
- Send prompt (with agent): POST `/session/:id/message` → body conforms to SessionPrompt.PromptInput; include `agent: string` and `model: { providerID, modelID }` plus `parts`. Sources:
  - Endpoint registration: packages/opencode/src/server/server.ts (operationId: session.prompt).
  - Schema (agent optional, defaults to build): packages/opencode/src/session/prompt.ts (PromptInput with `agent?: string`; createUserMessage uses `input.agent ?? "build"`).
- Event stream (already in use by eGUI): GET `/global/event` for SSE; no change needed for this feature.

## File-level Changes (high level)

- clients/egui/src/client/api.rs
  - Add list_agents(); extend message send call to accept an optional agent parameter when that path is added.
- clients/egui/src/app.rs
  - App state: agents: Vec<AgentInfo> and Tab { selected_agent: Option<String> }.
  - Fetch agents after ServerConnected; store and filter.
  - Render left SidePanel in session view; selection mutates tab.selected_agent.
  - In prompt send path, include agent (when available).
- clients/egui/src/types (new)
  - agent.rs with AgentInfo DTO.

## Edge Cases

- No agents returned: hide pane and fall back to server default behavior.
- Only subagents returned: show empty state with hint; let user enable "show subagents" to reveal them.
- Server error while fetching agents: show non-fatal toast; allow retry via Settings.

## Telemetry/Logs

- Log fetch errors and selection changes at DEBUG level to help diagnose issues.

## Testing

- Manual: start server, connect eGUI, verify agent list populates and persists per tab; switch agents and send prompts (when sending wired) to confirm server receives the selected agent.
- Mock: unit-test JSON decode of /agent response and filter logic.

## Follow-ups (optional)

- Persist per-tab agent choice across restarts.
- Color-coding message bubbles by agent.
- Keyboard shortcut to cycle agents.
