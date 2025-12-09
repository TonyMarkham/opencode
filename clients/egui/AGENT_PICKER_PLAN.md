# Agent Picker Implementation Plan

## Goals

- Surface server agents per tab with selectable active agent
- Default to primary agent (first non-subagent, fallback to "build") and keep selection stable across agent list changes when possible
- Avoid regressions to existing session/model handling and message flow

## Constraints & Notes

- Use GET `/agent` for discovery; prompt POST should carry `agent` alongside existing model payload
- Filter out `mode == "subagent"` unless user opts in via Settings
- Persist UI-only state in memory (pane collapse, per-tab selection); no disk persistence needed yet
- Show non-fatal feedback on fetch failure; allow retry

## Plan

1. **Data model**: Add `AgentInfo` DTO (name, mode, built_in flag, optional color/description) and extend `Tab` with `selected_agent: Option<String>`.
2. **Networking**: Add `OpencodeClient::list_agents()` to GET `/agent`; plumb optional `agent` into prompt send API shape for future wiring.
3. **State wiring**: On `UiMsg::ServerConnected`, fetch agents, store in app state, and track filtered vs full lists based on the subagent toggle; keep current selection if still present, else revert to default and surface a toast.
4. **UI panel**: Add left `SidePanel` in session view with collapsible state; list agents (radio-style), show built-in badge and optional color swatch, highlight selected, and append current agent name in tab header next to model.
5. **Selection logic**: Default new tabs to first primary agent (fallback "build"); clicking updates `tab.selected_agent` and logs at DEBUG.
6. **Prompt send**: When sending, include `agent: tab.selected_agent.unwrap_or(default_primary)` in request body (even if send path is not yet fully wired).
7. **Settings**: Add checkbox in Settings → UI for "Show subagents in agent list"; toggle immediately re-filters and revalidates selection.
8. **Error handling & feedback**: On fetch errors, log at DEBUG and show non-blocking toast with retry path; if agent list shrinks and selection disappears, reset to default and toast.
9. **Testing**: Add unit tests for `/agent` decode and filtering/default selection logic; perform manual verification with server to confirm list, selection, and prompt payload carry agent.

## Status

- Steps 1–8 implemented in `src/app.rs`, `src/client/api.rs`, `src/types/agent.rs`, and `src/types/models.rs`; agent picker UI and payload wiring are live.
- `cargo check` passes; warnings remain unchanged from upstream. Tests are still pending (step 9).
