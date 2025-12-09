# Server Version Feature

## Research Summary

- The opencode server embeds a `version` field into session objects when they are created: `Session.create` sets `version: Installation.VERSION` in `packages/opencode/src/session/index.ts`.
- The installed native binary also exposes a CLI `--version` flag (e.g. `/Users/tony/.opencode/bin/opencode --version` returned `1.0.134`).
- The HTTP API already returns session objects at `POST /session` and `GET /session` which include the session `version` field.
- No new server endpoint is required; the client can read the version from the session object the tab holds.

## Evidence / References

- Session sets version on creation: `packages/opencode/src/session/index.ts:178`.
- Server listen and routes: `packages/opencode/src/server/server.ts` (session routes under `/session`).
- Local binary used by PATH: `/Users/tony/.opencode/bin/opencode` (Mach-O executable; `--version` printed `1.0.134`).

## Implementation Plan

1. Add `version` to client session model
   - File: `clients/egui/src/client/api.rs`
   - Change: extend `SessionInfo` with `#[serde(default)] pub version: Option<String>`.

2. Propagate version into tab state
   - File: `clients/egui/src/app.rs`
   - Add `session_version: Option<String>` to `Tab` struct.
   - When `create_session` returns, include `info.version` in `UiMsg::SessionCreated` (update the variant and send site).
   - When handling `UiMsg::SessionCreated`, set `tab.session_version = info.version`.

3. Render version in tab footer
   - File: `clients/egui/src/app.rs`
   - In the tab footer rendering where server address is shown, append a separator and `v{version}` if present; otherwise omit.

4. Fallback (optional)
   - If `session_version` is None and you prefer not to rely on creation payload, add `get_session(session_id)` API method and fetch `/session/:id` once and cache result.

## UI label preference
- Recommend short label: `· v{version}` (compact).

---

If you want, I can implement these changes now. Reply with your preference for the label (`v{version}` vs `opencode {version}`) and I will apply edits to the repository.