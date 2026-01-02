# Next Session: Verify & Launch

## Quick Context

**What You Want:**

- Use your Anthropic Pro/Max subscription in the `egui` client.

**Current State:**

- **Client**: Fully implemented.
  - CLI: `cargo run -- --oauth` (interactive flow).
  - Auth: Saves to `auth.json`.
  - API: Sends `Authorization: Bearer <token>`.
- **Server**: Assumed ready.

---

## Your Mission

1. **Subscribe & Authenticate**:
   - Subscribe to Anthropic Pro/Max.
   - Run `cargo run -- --oauth` and follow the flow.
2. **Verify Usage**:
   - Restart the app (`cargo run`).
   - It should auto-select `claude-3-5-sonnet`.
   - Send a message.
   - **Verification**: Ensure the response works and you are not being billed for API usage (check Anthropic Console usage vs Subscription usage if possible).

---

## Troubleshooting

- If the server rejects the request (401/403) or ignores the token, **then and only then** investigate if the server configuration needs adjustment to enable the "Anthropic OAuth" feature that we know exists.

---

## Copy-Paste Starting Prompt

```
I'm ready to verify the Anthropic OAuth implementation.

1. I will run the client with `cargo run -- --oauth`.
2. I will authenticate with my new subscription.
3. I will test a chat message.

If it works, we are done.
```
