# Reasoning + Answer Chat Bubble (egui client notes)

This note is meant to give a future LLM instance concrete pointers for adding streamed **reasoning** and token "fuel gauge" UI to the egui client, using the existing opencode server APIs and events.

## Key server-side references

- HTTP server routes: `packages/opencode/src/server/server.ts`
  - `GET /global/event` — SSE endpoint used by the egui client to receive `GlobalEvent`s.
  - `GET /session/:id/message` — returns `MessageV2.WithParts[]` for a session.
- Message model: `packages/opencode/src/session/message-v2.ts`
  - `MessageV2.Info` — discriminated union of `User` and `Assistant`.
  - `MessageV2.WithParts` — `{ info: Info, parts: Part[] }`.
  - Important part types:
    - `TextPart` (type `"text"`)
    - `ReasoningPart` (type `"reasoning"`)
    - `ToolPart` (type `"tool"`)
  - Events:
    - `MessageV2.Event.PartUpdated` — bus event with `{ part, delta? }`, mirrored to SSE as `"message.part.updated"`.
- Token usage (per assistant message): `MessageV2.Assistant.tokens`
  - `input` — input/context tokens for this turn.
  - `output` — output tokens.
  - `reasoning` — reasoning tokens (for reasoning models).

## Key egui-side references

- SSE subscription: `clients/egui/src/client/events.rs`
  - `subscribe_global(base_url: &str)` opens `GET /global/event` and yields `GlobalEvent` values.
- App state and chat bubbles: `clients/egui/src/app.rs`
  - `Tab` (per-session UI state) holds:
    - `messages: Vec<DisplayMessage>` — the rendered chat bubbles.
    - `active_assistant: Option<String>` — currently streaming assistant message ID.
  - `DisplayMessage` currently is:

    ```rust
    #[derive(Clone)]
    struct DisplayMessage {
        message_id: String,
        role: String,
        text_parts: Vec<String>,
        tool_calls: Vec<ToolCall>,
    }
    ```

  - SSE handling in `OpenCodeApp::update` (search for `"message.updated"` and `"message.part.updated"`):
    - `UiMsg::GlobalEvent(serde_json::Value)` is matched.
    - For `"message.updated"` events, a `DisplayMessage` is created/updated per `message_id`.
    - For `"message.part.updated"` events, `text_parts` and `tool_calls` are updated incrementally.
  - Rendering of each bubble: `render_message(&mut self, ui: &mut egui::Ui, msg: &DisplayMessage, session_id: Option<&str>)`.
    - This is where user vs assistant layout and markdown rendering happens.

- HTTP message request shape: `clients/egui/src/types/models.rs`
  - `MessageRequest` is the body for `POST /session/:id/message`:

    ```rust
    pub struct MessageRequest {
        pub parts: Vec<MessagePart>,
        pub model: Option<ModelIdentifier>,
        pub agent: Option<String>,
    }

    #[serde(tag = "type")]
    pub enum MessagePart {
        #[serde(rename = "text")]
        Text { text: String },
        #[serde(rename = "file")]
        File { mime: String, filename: Option<String>, url: String },
    }
    ```

  - These `parts` map directly to `SessionPrompt.PromptInput.parts` and then to `MessageV2.Part`.

## Target behavior

**Goal:** For each assistant message, render a single bubble that contains:

1. A **collapsible reasoning section** (if any `ReasoningPart`s exist for that message).
2. The **final answer** text below it.
3. **Tool calls** as collapsible sections (already partially implemented).
4. Optional **token fuel gauge** (input/output/reasoning) per message.

## Data model changes in egui

Extend `DisplayMessage` (or wrap it) to track reasoning separately from answer text and tokens. One simple option:

```rust
#[derive(Clone)]
struct DisplayMessage {
    message_id: String,
    role: String,
    // Final answer text (assistant or user)
    text_parts: Vec<String>,
    // New: accumulated reasoning text for assistant messages
    reasoning_parts: Vec<String>,
    // Optional: token usage for this assistant turn
    tokens_input: Option<u64>,
    tokens_output: Option<u64>,
    tokens_reasoning: Option<u64>,
    // Existing tool call visualization
    tool_calls: Vec<ToolCall>,
}
```

Notes:

- Only assistant messages will use `reasoning_parts` and token fields.
- User messages can keep `reasoning_parts` and tokens as `None`/empty.

## Static rendering from history

When implementing history fetch (or if you add it later) via `GET /session/:id/message`:

1. Convert each `MessageV2.WithParts` into a `DisplayMessage`:
   - `message_id` = `info.id`.
   - `role` = `"assistant"` or `"user"` based on `info.role`.
   - For each `part` in `parts`:
     - If `part.type == "reasoning"`:
       - Push `part.text` into `reasoning_parts`.
     - If `part.type == "text"` and not synthetic/ignored:
       - Push `part.text` into `text_parts`.
     - If `part.type == "tool"`:
       - Map into a `ToolCall` entry (you already have this logic in the SSE path).
   - If `info.role == "assistant"`, copy token usage into the `DisplayMessage`:
     - `tokens_input = Some(info.tokens.input as u64)`
     - `tokens_output = Some(info.tokens.output as u64)`
     - `tokens_reasoning = Some(info.tokens.reasoning as u64)`
2. Insert into the appropriate `Tab.messages`.

This gives you a non-streaming baseline that matches what the SSE path will build incrementally.

## Streaming integration via SSE

In `OpenCodeApp::update` in `clients/egui/src/app.rs`, you already handle SSE `GlobalEvent`s containing an inner `payload` with an event name and data. The important bits:

- For `Some("message.updated")` you create/refresh a `DisplayMessage` entry.
- For `Some("message.part.updated")` you update `DisplayMessage.text_parts` and `tool_calls`.

To support reasoning:

1. In the `"message.part.updated"` branch, you already extract:
   - `message_id` from `part.get("messageID")`.
   - The `part` JSON payload.
2. Extend that handling to discriminate by `part["type"]`:
   - If `type == "reasoning"`:
     - Extract `text` from the part:

       ```rust
       if let Some(msg) = tab.messages.iter_mut().find(|m| m.message_id == mid) {
           if let Some(text) = part.get("text").and_then(|v| v.as_str()) {
               msg.reasoning_parts.push(text.to_owned());
           }
       }
       ```

     - If in the future the server adds a `delta` field, you can prefer appending `delta` over full `text`.

   - If `type == "text"` and `synthetic`/`ignored` are not set or false:
     - Keep existing behavior of appending to `text_parts`.
   - If `type == "tool"`:
     - Keep current tool update logic.

You do **not** need to understand the entire `MessageV2.Part` schema here; you only care about a small subset of fields from the JSON payload.

## Chat bubble rendering

In `render_message` in `clients/egui/src/app.rs`:

1. Detect assistant messages:

   ```rust
   match msg.role.as_str() {
       "assistant" => { /* assistant bubble layout */ }
       "user" => { /* user bubble layout */ }
       _ => { /* system/other */ }
   }
   ```

2. For assistant messages, inside the bubble:
   - **Reasoning section** (if any):

     ```rust
     if !msg.reasoning_parts.is_empty() {
         egui::collapsing_header::CollapsingState::load_with_default_open(
             ui.ctx(),
             format!("reasoning-{}", msg.message_id),
             false,
         )
         .show_header(ui, |ui| {
             ui.label("Reasoning");
         })
         .body(|ui| {
             let reasoning_text = msg.reasoning_parts.join("");
             ui.label(reasoning_text);
         });
     }
     ```

   - **Final answer**: render `msg.text_parts.join("")` using the existing markdown viewer (CommonMarkViewer) that you already use for assistant messages.

   - **Tool calls**: keep your existing collapsible tool call UI.

3. Optionally, show a small **fuel gauge** (e.g. a horizontal bar) using `tokens_input`, `tokens_output`, `tokens_reasoning`, together with the selected model’s limits from `ModelLimits` in `types/models.rs`.

## Fuel gauge details

You already have model limits via `ModelInfo.limit: ModelLimits { context, output }` in `clients/egui/src/types/models.rs`.

To build a simple per-message gauge:

1. Look up the current model’s `context` limit for the active tab.
2. For each assistant `DisplayMessage` with `tokens_input` set:
   - Compute `usage = tokens_input as f32 / context_limit as f32`.
3. Draw a small bar under the bubble:

   ```rust
   if let (Some(input), Some(context_limit)) = (msg.tokens_input, current_model_context_limit) {
       let usage = (input as f32 / context_limit as f32).clamp(0.0, 1.0);
       let desired_width = ui.available_width();
       let (rect, _) = ui.allocate_exact_size(
           egui::vec2(desired_width, 4.0),
           egui::Sense::hover(),
       );
       let bg = ui.visuals().extreme_bg_color;
       ui.painter().rect_filled(rect, 2.0, bg);
       let fill_rect = egui::Rect::from_min_max(
           rect.min,
           egui::pos2(rect.min.x + desired_width * usage, rect.max.y),
       );
       let fill_color = ui.visuals().widgets.active.bg_fill;
       ui.painter().rect_filled(fill_rect, 2.0, fill_color);
   }
   ```

If you want to incorporate `tokens_reasoning`, you can either:

- Show it as a secondary number (e.g. `R: 500 tokens`), or
- Use a different color overlay/stripe to indicate reasoning cost on top of the context bar.

## Summary for a future LLM

- **Server side**: reasoning and tokens are already modeled in `MessageV2` and surfaced via `/global/event` and `/session/:id/message`.
- **egui side**: the main work is:
  - Extending `DisplayMessage` with `reasoning_parts` and token fields.
  - Updating the SSE `"message.part.updated"` handler to recognize `part.type == "reasoning"` and append text for the correct `message_id`.
  - Adjusting `render_message` to show a collapsible reasoning section above the existing assistant answer and tool calls.
  - Optionally drawing a small per-message fuel gauge using `tokens.input` and the model’s context limit.
- This markdown (`clients/egui/ad-reasoning.md`) is intended as a starting point so you don’t have to rediscover file locations and message flow next time.
