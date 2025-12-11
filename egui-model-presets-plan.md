# EGUI Model Presets Implementation Plan

## 1. Goals and Constraints

- Allow users to define **multiple named model configurations** (presets) in the egui client.
- Each preset points to a base `provider/model_id` plus local metadata/settings.
- Persist presets in the **egui config** (`clients/egui/config/models.toml`), _not_ in the server config.
- Preserve existing behavior when no presets are defined.

---

## 2. Extend EGUI Model Config Data Structures

**Files:** `clients/egui/src/config/models.rs`

- Introduce a new `ModelPreset` struct, e.g.:
  - `id: String` (e.g. `"openai/gpt-5.1:high-reasoning"`).
  - `provider: String`.
  - `model_id: String`.
  - `label: String` (user-facing name shown in the UI).
  - Optional client-only metadata, e.g. `profile: Option<String>` (like `"high_reasoning"`) and/or `notes: Option<String>`.

- Extend `ModelsSection` to include:
  - `presets: Vec<ModelPreset>` in addition to the existing `curated: Vec<CuratedModel>` and `default_model`.

- Update `Default` impls so:
  - `presets` defaults to `Vec::new()`.
  - Existing behavior for `curated` and `default_model` remains unchanged.

- Ensure `ModelsConfig::load` / `save` still work:
  - If `presets` is absent from an older `models.toml`, it deserializes as an empty list.
  - Serialization includes `presets` only when non-empty.

---

## 3. Define `models.toml` Layout for Presets

**File:** `clients/egui/config/models.toml` (conceptual schema)

- Extend the TOML format to support a `[[models.presets]]` array alongside existing `[[models.curated]]`:

```toml
[models]
default_model = "openai/gpt-5.1"

[[models.curated]]
name = "GPT-5.1"
provider = "openai"
model_id = "gpt-5.1"

[[models.presets]]
id = "openai/gpt-5.1:high-reasoning"
provider = "openai"
model_id = "gpt-5.1"
label = "GPT-5.1 — High reasoning"
profile = "high_reasoning"

[[models.presets]]
id = "openai/gpt-5.1:cheap"
provider = "openai"
model_id = "gpt-5.1"
label = "GPT-5.1 — Cheap"
profile = "cheap"
```

- Ensure serde attributes on `ModelsSection` / `ModelPreset` line up with this layout (e.g., nested under `models` with `curated` and `presets`).

---

## 4. Surface Presets in the EGUI Model Picker

**File:** `clients/egui/src/app.rs`

- When building the model dropdown for each tab:
  - Load `ModelsConfig` once at startup (as currently done).
  - Construct a list of selectable entries that includes:
    - Raw models from `/config/providers` (current behavior).
    - Any matching `CuratedModel` entries from `ModelsConfig`.
    - Any `ModelPreset` entries from `ModelsConfig.models.presets`.

- For presets:
  - Display `label` (e.g. `"GPT-5.1 — High reasoning"`).
  - Optionally show the underlying `provider/model_id` as a secondary text.

- Keep `Tab.selected_model` as `Option<(String, String)>` (provider, model_id) so existing send logic continues to work unchanged.
  - Optionally add a separate field such as `selected_preset: Option<String>` to track which preset (if any) is active for the tab. This is purely for UI/metadata and future use.

---

## 5. Settings UI: Create/Edit/Delete Presets

**File(s):** settings panel in `clients/egui/src/app.rs` (or related UI modules)

- In the models section of the settings UI, add a **"Presets"** management area that:
  - Lists existing `ModelsConfig.models.presets` with:
    - `label`.
    - Underlying `provider/model_id`.
    - Optional `profile`/notes.

  - Allows adding a new preset:
    - Choose base provider/model from `/config/providers` (dropdown populated from `ProvidersResponse`).
    - Enter `label` and optional `profile` / notes.
    - Generate a default `id` (e.g. `"openai/gpt-5.1:custom-1"`) or allow user to specify.
    - Append a new `ModelPreset` to `ModelsConfig.models.presets` and call `ModelsConfig::save()`.

  - Allows editing an existing preset's `label` and metadata (keep provider/model fixed for now to avoid complexity).

  - Allows deleting a preset:
    - Remove from `ModelsConfig.models.presets`.
    - Save the updated config via `ModelsConfig::save()`.

- Backward‑compatible behavior:
  - If `models.toml` has only `curated` entries, the presets list is simply empty but functional.

---

## 6. Using Presets When Sending Messages

**Files:** `clients/egui/src/app.rs`, `clients/egui/src/types/models.rs`

- For now, keep server interaction compatible with the current API:
  - When a preset is selected, egui sends only the base model identifier:
    - `providerID = provider`
    - `modelID = model_id`

    using the existing `ModelIdentifier` in `types/models.rs::MessageRequest`.

  - Presets at this stage are **client-side semantics**:
    - They determine which `provider/model_id` is selected and how it is displayed.
    - They do **not** yet change server-side options (reasoning effort, max output tokens, etc.).

- Optionally, record the selected preset `id` on the tab state:
  - This lets egui:
    - Restore the same preset on restart.
    - Pass preset metadata onward in the future once the server can accept per-request model options.

---

## 7. Future Hook for Per-Request Options (No Implementation Yet)

- Anticipate a future where presets can carry **per-request model options** that the server understands.

- Extend `ModelPreset` to include an optional `options` map for that purpose (to be wired later), e.g.:

```rust
pub struct ModelPreset {
    pub id: String,
    pub provider: String,
    pub model_id: String,
    pub label: String,
    pub profile: Option<String>,
    // reserved for future server-aware overrides
    pub options: Option<std::collections::HashMap<String, serde_json::Value>>,
}
```

- For now, keep `options` unused in the egui client logic, or behind a feature flag, so behavior remains identical to current until the server supports these overrides.

---

## 8. Summary

This plan upgrades the egui client to support **multiple named model presets**:

- Presets live exclusively in the egui config (`models.toml`).
- The server continues to see only `provider/model_id` until per-request options are formally supported.
- The user gains the ability to define and choose between variants like:
  - `openai/gpt-5.1` (server default),
  - `openai/gpt-5.1:high-reasoning`,
  - `openai/gpt-5.1:cheap`,
    directly inside the egui settings and model picker.
