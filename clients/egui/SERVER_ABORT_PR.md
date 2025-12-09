# PR Draft: Fix abort handling to prevent cancelled turns from leaking

## Summary

- Make session abort fully prune the in-flight assistant turn so a subsequent prompt cannot inherit the cancelled user request.
- Ensure the `/session/:id/abort` endpoint waits for cancel + prune to finish before returning.

## Changes

- **`packages/opencode/src/session/prompt.ts`**
  - Added `cancelAndPrune(sessionID)` to abort, mark the in-flight assistant as `finish: "aborted"`, cancel any running tool parts, and then prune the aborted assistant and its parent user message (plus parts) from storage.
  - Added helpers to remove a message and its parts, and to log prune actions/failures.
  - **New code paths (high level):**
    - `markCancelled(sessionID)`: finds the in-flight assistant, sets `finish="aborted"`, stamps `time.completed`, and marks tool parts `status="cancelled"` with `time.end`.
    - `removeMessageWithParts(sessionID, messageID)`: deletes the message and all its parts from storage, with logging.
    - `pruneCancelled(sessionID)`: finds the latest aborted assistant, then removes it and its parent user message (and parts) from storage.
    - `cancelAndPrune(sessionID)`: aborts, calls `markCancelled`, then `pruneCancelled`.
- **`packages/opencode/src/server/server.ts`**
  - Updated the abort route to `await SessionPrompt.cancelAndPrune(...)` so the prune completes before the API returns.

## Rationale

- Previously, abort only flipped the abort controller and set status idle; the unfinished assistant/user turn remained in history. The next prompt could therefore act on the cancelled request and emit tool/permission events under a new assistant ID. Pruning the cancelled turn removes it from the model context and stops tool/permission emissions tied to that turn.

## Code snippets (with reasoning)

```ts
// session/prompt.ts — mark the in-flight assistant aborted and cancel tools
await Session.updateMessage({
  ...assistant,
  finish: "aborted",
  time: { ...assistant.time, completed: now },
})
// cancel any running tool parts
await Session.updatePart({
  ...part,
  state: {
    ...part.state,
    status: "cancelled",
    time: { ...time, end: now, start: time.start ?? now },
  },
} as MessageV2.ToolPart)
```

_Reasoning:_ Set the unfinished assistant to `aborted` and end any tool parts so the turn is terminal and won’t emit further events.

```ts
// session/prompt.ts — delete aborted assistant and its parent user message
const parts = await Storage.list(["part", messageID])
for (const p of parts) await Storage.remove(p)
await Session.removeMessage({ sessionID, messageID })
```

_Reasoning:_ Physically remove the aborted assistant and its parent user message (and their parts) from storage so the cancelled turn is pruned from the model history and cannot be used in the next prompt.

```ts
// session/prompt.ts — pruneCancelled ties it together
if (targetAssistant) {
  await removeMessageWithParts(sessionID, targetAssistant.id)
  if (parentID) await removeMessageWithParts(sessionID, parentID)
}
```

_Reasoning:_ Once an aborted assistant is found, prune it and its parent user to fully excise the cancelled turn.

```ts
// server/server.ts — abort route awaits cancel+prune
await SessionPrompt.cancelAndPrune(c.req.valid("param").id)
```

_Reasoning:_ Ensure the abort API call waits for cancel + prune to finish before returning, so the next turn starts with the cancelled turn already removed.

## Testing

- Manual: start server after rebuild and reproduce Stop → next prompt; tool/permission emissions from the cancelled turn should no longer appear on the next prompt.
- (Automated tests not run here; `bun` unavailable in this environment.)
