# Model SDK Configuration Plan

## Goals

- Expose a **typed, documented** per-model configuration layer that maps cleanly onto the underlying AI SDK options.
- Avoid breaking existing behavior (hard-coded defaults in `provider/transform.ts` continue to work until overridden).
- Make it easy to reason about and document which options are valid for which providers/models.

## Current State

- Provider/model metadata comes from **models.dev** (`ModelsDev.Model`) plus user overrides in `~/.config/opencode/config.json` under `provider.<id>.models.<modelId>`.
- Per-model `options` is currently typed as `record<string, any>` and used ad-hoc in `provider/transform.ts` (e.g., `reasoningEffort`, `textVerbosity`, `thinkingConfig`, `include`, `reasoningSummary`, `maxOutputTokens`).
- The underlying SDK (OpenAI-compatible, Anthropic, Google, etc.) already defines richer option types, but opencode does not surface them in a structured way.

## High-Level Design

1. Introduce a `ModelOptions` schema in opencode that describes **supported** per-model options, with provider-aware typing where practical.
2. Thread `ModelOptions` into:
   - `ModelsDev.Model` (optional, so models.dev data continues to validate).
   - `Config.Provider.models[modelId]` in `config.ts`, so `config.json` is validated.
   - `Provider.Model` in `provider.ts`, so `provider/transform.ts` gets typed access instead of `(model as any).options`.
3. Update `provider/transform.ts` to:
   - Read from `model.options` (typed) first.
   - Fall back to existing per-model defaults for GPT-5, Gemini, Anthropic, etc.
   - Pass options through to the SDK in a consistent shape (e.g., `openai.reasoning.effort`, `google.thinkingConfig`, `max_output_tokens`).
4. Document the supported options and how they map to provider-specific behavior.

## Proposed `ModelOptions` Surface (Initial)

Start with a conservative, high-value subset that we already touch implicitly:

- **Core generation**
  - `maxOutputTokens?: number` — upper bound, clamped to model/global caps.
  - `temperatureOverride?: number` — optional per-model override (falls back to provider default / global setting).
  - `topPOverride?: number` — optional per-model override.
- **Reasoning / chain-of-thought**
  - `reasoningEffort?: "minimal" | "low" | "medium" | "high"` (OpenAI / gpt-5 family, mapped to SDK reasoning options).
  - `reasoningSummary?: "auto" | "off"` (OpenAI reasoning summary strategy).
  - `include?: string[]` (e.g., `"reasoning.encrypted_content"` for GPT-5.1).
- **Gemini thinking**
  - `thinkingConfig?: { includeThoughts?: boolean; thinkingBudget?: number }` (Google / Gemini-only).
- **Storage / misc**
  - `store?: boolean` (Codex/GPT-5 storage preference when applicable).

These live under:

```jsonc
{
  "provider": {
    "openai": {
      "models": {
        "gpt-5.1": {
          "options": {
            "maxOutputTokens": 400,
            "reasoningEffort": "medium",
            "textVerbosity": "medium",
            "include": ["reasoning.encrypted_content"],
            "reasoningSummary": "auto",
          },
        },
      },
    },
  },
}
```

## Implementation Phases

### Phase 1: Schema + Plumbing (Non-Breaking)

1. **Schema extensions**
   - Extend `ModelsDev.Model` with an optional `options?: ModelOptionsRaw` field.
   - Add a `ModelOptions` zod schema in `config/config.ts` and plug it into `Config.Provider.models[modelId].options`.
2. **Provider.Model wiring**
   - In `provider/provider.ts`, ensure `Model.options` carries the merged options from:
     - models.dev data (`ModelsDev.Model.options`).
     - `config.json` overrides (`Config.Provider.models[modelId].options`).
3. **Transform updates**
   - Refactor `provider/transform.ts` to:
     - Replace `(model as any).options` with a typed `model.options`.
     - Keep existing defaults in place when `model.options` does not specify a value.
   - Keep `maxOutputTokens` behavior as: config override → Anthropic thinking math → current caps.
4. **Validation**
   - Run existing tests and add one or two small tests that assert:
     - A config-provided `maxOutputTokens` is honored and clamped.
     - A config-provided `reasoningEffort` for GPT-5.1 replaces the hard-coded default.

### Extras Pattern (gltf-style)

To support provider-specific or experimental options (e.g., DeepSeek, Kimi K2, gateway providers) without polluting the typed `options` surface:

- Introduce `extras?: Record<string, unknown>` alongside `options` at both provider and model levels in the config schema.
- Thread `extras` through `ModelsDev.Model`, `Config.Provider`, and `Provider.Model` unchanged.
- Treat `options` as the **blessed, documented** surface that opencode understands and maps into the SDK.
- Treat `extras` as a safe POC / extension bag:
  - Provider adapters may opt-in to read keys from `extras` and translate them to SDK-specific payloads.
  - When an `extras` key proves broadly useful, we can promote it into `options` and document it.

### Phase 2: SDK Mapping & Provider-Specific Rules

1. **OpenAI / GPT-5**
   - Map `ModelOptions` to OpenAI SDK options (e.g., `reasoning.effort`, `reasoning.summary`, `include`).
   - Ensure non-reasoning models ignore reasoning-only options cleanly.
2. **Google / Gemini**
   - Map `thinkingConfig` into the Google SDK structures.
   - Ensure non-Gemini providers ignore `thinkingConfig`.
3. **Anthropic**
   - Confirm `maxOutputTokens` interacts correctly with Anthropic thinking budget (existing `maxOutputTokens` logic already handles this, but we should document it).
4. **Defensive checks**
   - Optionally warn (log) when clearly invalid combinations are configured (e.g., `thinkingConfig` on OpenAI-only models), without hard failing.

### Phase 3: Documentation

1. **SDK Options README** (new markdown under `specs/` or `packages/opencode`):
   - Describe `ModelOptions` as the authoritative per-model config surface.
   - For each option:
     - Name and type.
     - Which providers/models it applies to.
     - How it maps into the underlying SDK.
     - Any interactions with costs/limits (e.g., reasoning tokens vs context cap, Anthropic thinking budget).
   - Provide 3–5 concrete config examples:
     - GPT-5.1 tuned for visible reasoning.
     - Gemini 2.5 Pro with and without `includeThoughts`.
     - A conservative “cheap” small model config.
2. **Config schema reference**
   - Update general configuration docs to reference `provider.<id>.models.<modelId>.options` and link to the SDK options README.

## Non-Goals (For Now)

- Supporting **every** possible provider/SDK option out of the box.
- Validating provider-specific constraints at config-parse time (we’ll start with best-effort typing and documented behavior, and add more validation later if needed).
- Changing existing default behavior for users who do not specify any `options` in their config.

## Longer-Term Goal

- Gradually move **all** hard-coded per-model configuration (e.g., GPT-5/Gemini reasoning defaults, thinkingConfig, includes) out of source code and into:
  - models.dev metadata, and/or
  - default config that is merged like user `config.json`.
- Once those defaults live in config/metadata, simplify `provider/transform.ts` so it:
  - Reads only from `model.options` / `model.extras` and generic capabilities,
  - Contains no `if (model.api.id.includes("gpt-5"))` / `if (model.providerID === "google")` branches,
  - Acts purely as a mapping layer from opencode’s `ModelOptions`/`extras` into the SDK.
