# Dynamic Model Selection for OpenCode EGUI

Status: **✅ IMPLEMENTED** (2025-12-08)

## Implementation Status

**Core Feature: COMPLETE and FUNCTIONAL**

- ✅ Phase 1: Config Infrastructure & Build System
- ✅ Phase 2: Async API Key Sync
- ✅ Phase 3: Provider API Client & Discovery
- ✅ Phase 4: Model Management UI
- ✅ Phase 5: Per-Tab Model Selection
- 🔲 Phase 6: Testing & Polish (optional enhancements)

**What Works:**
- Provider configs fully dynamic (TOML-based, zero hardcoded logic)
- Model discovery by calling provider APIs directly (OpenAI, Anthropic, Google, OpenRouter)
- Search/filter discovered models
- Curated models list (add/remove via UI)
- Per-tab model dropdown with status indicators
- Model selection persists per conversation tab
- Prompts sent with selected model to server
- Auth keys sync to server on startup
- Extra headers support (e.g., Anthropic's version header)

**Known Limitations:**
- UI is functional but minimal (Phase 6 polish not implemented)
- No retry logic for failed API calls
- No detailed model metadata display (context window, capabilities)
- No usage statistics or cost tracking

## Goals
- Enable per-tab model selection from a curated list
- Support multiple LLM providers (OpenAI, Anthropic, Google, OpenRouter)
- Fetch available models dynamically from each provider's API
- Provide intuitive UI for discovering and adding models to curated list
- Allow quick model switching per conversation tab

## Non-Goals
- Automatic model selection based on task type
- Model performance benchmarking
- Cost optimization recommendations
- Multi-model orchestration (using multiple models in single conversation)

## Architecture Overview

**Key Principle:** EGUI discovers models by calling provider APIs directly. Server handles only LLM inference.

### Division of Responsibilities

**EGUI Client:**
- Stores provider configs in `config/models.toml` (API endpoints, auth types, response parsing)
- Stores API keys in `.env`
- Calls provider APIs directly for model discovery (OpenAI, Anthropic, Google, etc.)
- Maintains curated models list locally
- Syncs API keys to server on startup
- Sends prompts to server with `provider/model_id`

**OpenCode Server:**
- Receives API keys from EGUI via `PUT /auth/:id`
- Stores keys in `~/.local/share/opencode/auth.json`
- Receives prompts with `provider/model_id` from EGUI
- Uses stored API keys to call provider APIs for inference
- Returns LLM responses to EGUI

**Server provider endpoints (`/config/providers`, `/provider`) are NOT used** - they exist but are optional. EGUI discovers models independently.

### Component Structure

```
EGUI Executable Directory:
  opencode-egui           - Executable binary
  .env                    - API keys (gitignored, user-specific)
  config/
    models.toml          - Curated model list + UI preferences
  
EGUI Client Source:
  src/config/models.rs    - Load/save config/models.toml
  src/client/server.rs    - API calls to OpenCode server
  src/startup/auth.rs     - Async API key sync on startup
  src/ui/settings.rs      - Model discovery and management UI
  src/ui/tab.rs          - Model dropdown in tab header
  src/types/models.rs     - Model data structures

OpenCode Server:
  ~/.local/share/opencode/auth.json  - Stored API credentials (managed by server)
  Existing /config/providers endpoint - Returns available models
  Existing PUT /auth/:id endpoint     - Receives API keys from EGUI
```

### Build System

**Development builds** (`make dev`):
- Copy existing `.env` file to build output directory (if present)
- Allows developers to use real API keys

**Production builds** (`make build` / `cargo make build`):
- Copy `.env.example` to build output directory
- Users fill in their own API keys after installation

### Startup Flow

**App Launch (Non-Blocking):**
1. EGUI starts, UI renders immediately
2. Background task spawns to sync API keys:
   - Load `.env` file
   - For each API key found, call `PUT /auth/{providerID}` on server
   - Server stores credentials in `~/.local/share/opencode/auth.json`
3. Model dropdown shows spinner during sync
4. After sync completes:
   - Fetch available models from server via `GET /config/providers`
   - Populate model dropdown with curated list
   - Show checkmark or success indicator
5. If sync fails:
   - Show error icon in model dropdown
   - Display helpful error message (server offline, invalid key, etc.)

### Data Flow

**Adding a Model:**
1. User opens Settings → "Add Model"
2. EGUI shows provider list from `config/models.toml`
3. User selects a provider (e.g., "OpenAI")
4. EGUI calls provider's API directly:
   - Reads `api_key_env` from provider config → gets key from `.env`
   - Builds HTTP request using `auth_type` (bearer/header/query_param)
   - Calls `models_url` endpoint
   - Parses response using `response_format` config
5. EGUI displays searchable list of discovered models
6. User selects model → added to `[[models.curated]]` in `config/models.toml`
7. Curated list saved locally

**Using a Model:**
1. User clicks model dropdown in tab header
2. Shows curated model list from `config/models.toml`
3. User selects model → tab stores selection
4. When user sends a prompt:
   - EGUI includes model in API request: `POST /session/{id}/message`
   - Request body: `{ "parts": [...], "model": { "providerID": "openai", "modelID": "gpt-4" } }`
5. Server receives request and uses specified model for inference
6. Server looks up stored credentials from `~/.local/share/opencode/auth.json`
7. Server makes LLM API call with those credentials

**Key Point:** EGUI discovers models by calling provider APIs directly. Server provider endpoints (`/config/providers`, `/provider`) are NOT needed for this workflow.

### Complete End-to-End Workflow

**1. Initial Setup (User):**
- User creates `.env` file with API keys:
  ```bash
  OPENAI_API_KEY=sk-...
  ANTHROPIC_API_KEY=sk-ant-...
  ```
- Default `config/models.toml` ships with provider configs pre-defined

**2. App Startup (Async):**
- EGUI loads `config/models.toml` to read provider configs
- EGUI loads `.env` to read API keys
- Background task:
  ```
  For each provider in config/models.toml:
    - Read api_key_env field (e.g., "OPENAI_API_KEY")
    - Get key from .env
    - Call PUT /auth/{provider_name} with { "type": "api", "key": "..." }
  ```
- Model dropdown shows spinner during sync
- After sync: dropdown shows curated models from `[[models.curated]]`

**3. User Adds Model:**
- User opens Settings → "Add Model"
- EGUI shows provider list from `[[providers]]` in config
- User selects provider (e.g., "OpenAI")
- EGUI:
  ```
  1. Get provider config from [[providers]] where name = "openai"
  2. Read api_key_env = "OPENAI_API_KEY"
  3. Get key from .env: OPENAI_API_KEY
  4. Build HTTP request:
     - URL: providers.models_url
     - Auth: Based on providers.auth_type (bearer/header/query_param)
     - Header/Param: API key
  5. Parse response using providers.response_format:
     - Extract models array from models_path
     - For each model:
       - Get ID from model_id_field
       - Get name from model_name_field
       - Strip prefix if model_id_strip_prefix set
  ```
- Display searchable model list
- User selects model → EGUI adds to `[[models.curated]]` in config/models.toml
- File saved

**4. User Sends Prompt with Model:**
- User selects model from dropdown (reads from `[[models.curated]]`)
- Tab stores: `{ provider: "openai", model_id: "gpt-4" }`
- User types prompt → sends
- EGUI calls:
  ```
  POST /session/{id}/message
  {
    "parts": [{ "type": "text", "text": "Hello" }],
    "model": {
      "providerID": "openai",
      "modelID": "gpt-4"
    }
  }
  ```
- Server:
  ```
  1. Receives request
  2. Looks up auth for "openai" in ~/.local/share/opencode/auth.json
  3. Finds API key (synced in step 2)
  4. Calls OpenAI API with that key
  5. Returns response to EGUI
  ```

**5. Error Scenarios:**
- API key invalid: Provider API returns 401 during discovery → EGUI shows error
- Model not found: Server returns error when prompt sent → EGUI displays error
- Network failure: EGUI shows retry button

## Configuration

### .env (API Keys)

Stored adjacent to the EGUI executable. **Never commit this file.**

```bash
# API Keys - EGUI syncs these to the server on startup
OPENAI_API_KEY=sk-...
ANTHROPIC_API_KEY=sk-ant-...
GOOGLE_API_KEY=...
OPENROUTER_API_KEY=...
```

**Key Sync Process:**
- On app startup, EGUI reads `.env`
- Asynchronously sends each key to server: `PUT /auth/{providerID}`
- Server stores credentials in `~/.local/share/opencode/auth.json`
- Server uses these credentials for all LLM inference

**Mapping Provider ID from Env Variable:**
- `OPENAI_API_KEY` → `openai`
- `ANTHROPIC_API_KEY` → `anthropic`
- `GOOGLE_API_KEY` → `google`
- `OPENROUTER_API_KEY` → `openrouter`
- Pattern: lowercase, strip `_API_KEY` suffix

### config/models.toml (Provider Configuration + Curated Models)

Stored in `config/` directory adjacent to the EGUI executable.

**Architecture Note:** While the server handles actual provider authentication and model inference, this file defines HOW to discover models and map API keys. This enables dynamic provider support without hardcoding provider logic in EGUI source code.

#### Full Schema Definition

```toml
# ============================================================================
# PROVIDER CONFIGURATIONS
# ============================================================================
# Dynamic provider definitions - no hardcoded provider logic in source code
# To add a new provider: just add a [[providers]] entry

[[providers]]
name = "openai"
display_name = "OpenAI"
api_key_env = "OPENAI_API_KEY"           # Links to .env variable
models_url = "https://api.openai.com/v1/models"
auth_type = "bearer"                      # Authorization: Bearer <key>

# Response parsing configuration - how to extract model data from API response
[providers.response_format]
models_path = "data"                      # JSONPath to model array
model_id_field = "id"                     # Field containing model ID
model_name_field = "id"                   # Field for display name
description_field = null                  # No description in response
context_window_field = null               # Not provided by API
capabilities_object = null                # No capabilities object

[[providers]]
name = "anthropic"
display_name = "Anthropic"
api_key_env = "ANTHROPIC_API_KEY"
models_url = "https://api.anthropic.com/v1/models"
auth_type = "header"
auth_header = "x-api-key"                 # Custom header name

[providers.response_format]
models_path = "data"
model_id_field = "id"
model_name_field = "display_name"
description_field = "description"
context_window_field = "context_window"
# Nested capabilities object
capabilities_object = "capabilities"
capabilities_mapping = { vision = "vision", tools = "tool_use", streaming = "streaming" }

[[providers]]
name = "google"
display_name = "Google Gemini"
api_key_env = "GOOGLE_API_KEY"
models_url = "https://generativelanguage.googleapis.com/v1/models"
auth_type = "query_param"                 # Key goes in URL query string
auth_param = "key"

[providers.response_format]
models_path = "models"
model_id_field = "name"                   # Format: "models/gemini-pro"
model_id_strip_prefix = "models/"         # Strip this prefix
model_name_field = "displayName"
description_field = "description"
context_window_field = "inputTokenLimit"
# Google uses supportedGenerationMethods array
capabilities_array = "supportedGenerationMethods"
capabilities_mapping = { vision = "generateContent", tools = "generateContent" }

[[providers]]
name = "openrouter"
display_name = "OpenRouter"
api_key_env = "OPENROUTER_API_KEY"
models_url = "https://openrouter.ai/api/v1/models"
auth_type = "bearer"

[providers.response_format]
models_path = "data"
model_id_field = "id"
model_name_field = "name"
description_field = "description"
context_window_field = "context_length"
# OpenRouter capabilities are top-level booleans
capabilities_fields = { vision = "supports_vision", tools = "supports_tools" }

# ============================================================================
# CURATED MODELS
# ============================================================================
# User's favorite models - shown in tab dropdown

[models]
default_model = "openai/gpt-4"           # Default for new tabs

[[models.curated]]
name = "GPT-4"
provider = "openai"
model_id = "gpt-4"

[[models.curated]]
name = "Claude 3.5 Sonnet"
provider = "anthropic"
model_id = "claude-3-5-sonnet-20241022"

[[models.curated]]
name = "Gemini 2.0 Flash"
provider = "google"
model_id = "gemini-2.0-flash-exp"

[[models.curated]]
name = "DeepSeek V3 (via OpenRouter)"
provider = "openrouter"
model_id = "deepseek/deepseek-chat"
```

#### Field Reference

**`[[providers]]` Section (Provider Configuration):**

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `name` | string | Yes | Unique provider identifier (lowercase, no spaces) |
| `display_name` | string | Yes | Human-readable name shown in UI |
| `api_key_env` | string | Yes | Environment variable name from `.env` file |
| `models_url` | string | Yes | Provider's API endpoint for listing models |
| `auth_type` | string | Yes | Authentication method: `"bearer"`, `"header"`, or `"query_param"` |
| `auth_header` | string | Conditional | Required if `auth_type = "header"`. Custom header name (e.g., `"x-api-key"`) |
| `auth_param` | string | Conditional | Required if `auth_type = "query_param"`. Query parameter name (e.g., `"key"`) |

**Authentication Types:**
- `bearer`: Standard OAuth bearer token in `Authorization: Bearer <key>` header
- `header`: Custom header specified by `auth_header` field
- `query_param`: API key in URL query string (e.g., `?key=<api_key>`)

**`[providers.response_format]` Section (Response Parsing):**

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `models_path` | string | Yes | JSONPath to array of models in provider response |
| `model_id_field` | string | Yes | Field name containing the model identifier |
| `model_id_strip_prefix` | string | No | Optional prefix to remove from model IDs |
| `model_name_field` | string | Yes | Field name for human-readable model name |
| `description_field` | string/null | No | Field name for model description (null if unavailable) |
| `context_window_field` | string/null | No | Field name for context window size |
| `capabilities_object` | string/null | No | Nested object containing capability flags |
| `capabilities_array` | string/null | No | Array field containing capability strings |
| `capabilities_fields` | object/null | No | Map of top-level boolean capability fields |
| `capabilities_mapping` | object/null | No | Map internal names to provider-specific field names |

**`[models]` Section (UI Preferences):**
- `default_model` (string, required): Default model for new tabs. Format: `"provider/model_id"`

**`[[models.curated]]` Entries (Favorite Models):**
- `name` (string, required): Human-readable display name shown in model dropdown
- `provider` (string, required): Provider ID (must match a `[[providers]]` entry's `name`)
- `model_id` (string, required): Model identifier used in API requests to server

#### API Key Mapping

The `api_key_env` field links each provider to its API key in `.env`:

```toml
# In config/models.toml
[[providers]]
name = "openai"
api_key_env = "OPENAI_API_KEY"
```

```bash
# In .env
OPENAI_API_KEY=sk-...
```

On startup, EGUI:
1. Loads `config/models.toml` to get provider definitions
2. For each provider, reads API key from `.env` using `api_key_env` field
3. Syncs key to server via `PUT /auth/{provider_name}` with `{ "type": "api", "key": "..." }`

#### Notes
- Model metadata (description, capabilities, context window, cost) comes from server at runtime
- Provider configurations enable dynamic model discovery without hardcoding in source
- To add a new provider: add `[[providers]]` entry + corresponding `_API_KEY` in `.env`
- Invalid provider/model_id combinations are validated against server's provider list
- Empty curated list prompts user to add models via discovery UI

## Server API Integration

### Authentication Endpoints (Existing)

#### PUT /auth/:id
EGUI uses this to sync API keys to the server.

**Request:**
```bash
PUT /auth/openai
Content-Type: application/json

{
  "type": "api",
  "key": "sk-..."
}
```

**Response:**
```json
true
```

### Provider Endpoints (Optional - Not Used in Core Workflow)

**Note:** These endpoints exist on the server but are NOT needed for the dynamic model selection feature. EGUI discovers models by calling provider APIs directly.

These are documented for reference only:

#### GET /config/providers (Optional)
Returns only authenticated providers with API keys stored on server.

#### GET /provider (Optional)
Returns all providers from server's database (models.dev).

**Why not used:**
- EGUI calls provider APIs directly for fresh model discovery
- Server database may be outdated (doesn't have newly released models)
- Direct API calls validate API keys work at discovery time

### Session Endpoints (Existing)

#### POST /session/:id/message
Send a prompt with a specific model.

**Request:**
```json
{
  "parts": [
    {
      "type": "text",
      "text": "Hello!"
    }
  ],
  "model": {
    "providerID": "openai",
    "modelID": "gpt-4"
  }
}
```

Server uses the specified model and looks up stored credentials from `~/.local/share/opencode/auth.json`.

## UI Design

### Settings Window - "Add Model" Workflow

**Step 1: Select Provider**
```
┌─────────────────────────────────────┐
│  Add Model                      [X] │
├─────────────────────────────────────┤
│                                     │
│  Select a provider:                 │
│                                     │
│  ○ OpenAI                           │
│  ○ Anthropic                        │
│  ○ Google                           │
│  ○ OpenRouter                       │
│                                     │
│               [Cancel]  [Next]      │
└─────────────────────────────────────┘
```

**Step 2: Search and Select Model**
```
┌─────────────────────────────────────────────┐
│  Add Model from OpenAI              [X]     │
├─────────────────────────────────────────────┤
│                                             │
│  Search: [gpt-5                       ]     │
│                                             │
│  ┌─────────────────────────────────────┐   │
│  │ ☐ gpt-5.1                           │   │
│  │   Latest model - 128K context       │   │
│  │                                     │   │
│  │ ☐ gpt-5.1-codex                     │   │
│  │   Optimized for code - 128K         │   │
│  │                                     │   │
│  │ ☐ gpt-5-mini                        │   │
│  │   Smaller, faster - 128K            │   │
│  │                                     │   │
│  └─────────────────────────────────────┘   │
│                                             │
│            [Back]  [Add Selected]           │
└─────────────────────────────────────────────┘
```

**Step 3: Confirm Addition**
```
┌─────────────────────────────────────┐
│  Model Added                    [X] │
├─────────────────────────────────────┤
│                                     │
│  ✓ GPT-5.1 added to your models     │
│                                     │
│  You can now select it from any     │
│  tab's model dropdown.              │
│                                     │
│                    [OK]             │
└─────────────────────────────────────┘
```

### Settings Window - Manage Curated Models

```
┌─────────────────────────────────────────────┐
│  Settings                           [X]     │
├─────────────────────────────────────────────┤
│                                             │
│  Models                                     │
│                                             │
│  Your curated models:                       │
│                                             │
│  GPT-5.1 (OpenAI)                    [×]    │
│  Claude 4.5 Sonnet (Anthropic)       [×]    │
│  Gemini 3 Pro (Google)               [×]    │
│                                             │
│  [+ Add Model]                              │
│                                             │
│  Default model for new tabs:                │
│  [GPT-5.1                        ▼]         │
│                                             │
│                           [Save]  [Cancel]  │
└─────────────────────────────────────────────┘
```

### Tab Header - Model Selector

```
┌─────────────────────────────────────────────┐
│  New Chat          [GPT-5.1 ▼]         [×]  │
├─────────────────────────────────────────────┤
│                                             │
│  User: Hello!                               │
│                                             │
│  Assistant: Hi! How can I help?             │
│                                             │
└─────────────────────────────────────────────┘
```

**Dropdown menu:**
```
┌─────────────────────────┐
│ ● GPT-5.1               │
│   Claude 4.5 Sonnet     │
│   Gemini 3 Pro          │
├─────────────────────────┤
│   Manage Models...      │
└─────────────────────────┘
```

## Implementation Plan

### Phase 1: Config Infrastructure & Build System ✅ COMPLETE
1. Create `config/models.toml` with example curated models
2. Create `.env.example` with placeholder API keys
3. Update Makefile/Cargo.make:
   - Dev builds: copy `.env` to output directory (if exists)
   - Production builds: copy `.env.example` to output directory
   - Always copy `config/` directory to output
4. Add dependencies:
   - `toml` crate for parsing models.toml
   - `dotenvy` crate for loading .env (use this over deprecated `dotenv`)
   - `reqwest` for HTTP calls to server
   - `serde_json` for JSON parsing
   - `tokio` for async runtime

### Phase 2: Async API Key Sync ✅ COMPLETE
1. Create `src/startup/auth.rs`:
   ```rust
   pub struct AuthSyncState {
       pub status: AuthSyncStatus,
       pub synced_providers: Vec<String>,
       pub failed_providers: Vec<(String, String)>, // (provider, error)
   }
   
   pub enum AuthSyncStatus {
       NotStarted,
       InProgress,
       Complete,
       Failed(String),
   }
   
   pub async fn sync_api_keys_to_server(server_url: &str) -> AuthSyncState {
       // Load .env
       // For each *_API_KEY, extract provider name
       // Call PUT /auth/{provider} with { "type": "api", "key": "..." }
       // Return status
   }
   ```
2. Update app startup:
   - Spawn async task for `sync_api_keys_to_server()`
   - Store `AuthSyncState` in app state
   - UI can check sync status before showing model dropdown
3. Provider name extraction:
   - `OPENAI_API_KEY` → `openai` (lowercase, strip suffix)
   - Handle known providers: OpenAI, Anthropic, Google, OpenRouter, etc.

### Phase 3: Provider API Client & Discovery ✅ COMPLETE
1. Create `src/client/providers.rs`:
   ```rust
   pub struct ProviderClient {
       client: reqwest::Client,
   }
   
   impl ProviderClient {
       /// Discover models from a provider's API using config from models.toml
       pub async fn discover_models(
           &self,
           provider_config: &ProviderConfig,
           api_key: &str,
       ) -> Result<Vec<DiscoveredModel>> {
           // 1. Build HTTP request based on auth_type
           let request = match provider_config.auth_type {
               AuthType::Bearer => {
                   self.client.get(&provider_config.models_url)
                       .header("Authorization", format!("Bearer {}", api_key))
               },
               AuthType::Header { name } => {
                   self.client.get(&provider_config.models_url)
                       .header(name, api_key)
               },
               AuthType::QueryParam { name } => {
                   let url = format!("{}?{}={}", provider_config.models_url, name, api_key);
                   self.client.get(&url)
               },
           };
           
           // 2. Make request
           let response = request.send().await?;
           let json: serde_json::Value = response.json().await?;
           
           // 3. Parse using response_format config
           parse_provider_response(json, &provider_config.response_format)
       }
   }
   ```
2. Create `src/client/server.rs`:
   ```rust
   pub struct ServerClient {
       base_url: String,
       client: reqwest::Client,
   }
   
   impl ServerClient {
       pub async fn sync_auth(&self, provider: &str, key: &str) -> Result<()> {
           // PUT /auth/{provider}
       }
       
       pub async fn send_message(&self, session_id: &str, request: MessageRequest) -> Result<()> {
           // POST /session/{id}/message
       }
   }
   ```
3. Create `src/types/models.rs`:
   ```rust
   struct ProviderConfig {
       name: String,
       display_name: String,
       api_key_env: String,
       models_url: String,
       auth_type: AuthType,
       response_format: ResponseFormat,
   }
   
   struct CuratedModel {
       name: String,
       provider: String,
       model_id: String,
   }
   
   struct DiscoveredModel {
       id: String,
       name: String,
       description: Option<String>,
   }
   ```
4. Create `src/config/models.rs`:
   - Load config/models.toml at startup
   - Parse [[providers]] sections
   - Load [[models.curated]] list
   - Save updated curated list back to TOML

### Phase 4: Model Management UI ✅ COMPLETE
1. Create ModelDiscoveryWindow:
   - Step 1: Show provider list from `[[providers]]` in config/models.toml
   - Step 2: User selects provider (e.g., "OpenAI")
   - Step 3: EGUI calls provider API directly using ProviderClient:
     ```rust
     let provider_config = load_provider_config("openai")?;
     let api_key = env::var(&provider_config.api_key_env)?;
     let models = provider_client.discover_models(&provider_config, &api_key).await?;
     ```
   - Step 4: Display searchable/filterable model list
   - Step 5: User selects model → add to `[[models.curated]]` in config/models.toml
2. Add "Models" section to Settings window:
   - Display curated models from config/models.toml
   - Show provider and model ID for each
   - "Add Model" button opens ModelDiscoveryWindow
   - Remove button for each curated model
   - Default model selector dropdown
3. Implement save/persist:
   - Write updated curated list back to config/models.toml
   - Validate TOML before saving
   - Reload model list after save

### Phase 5: Per-Tab Model Selection ✅ COMPLETE
1. Add `selected_model` field to Tab struct:
   ```rust
   struct Tab {
       // ... existing fields
       selected_model: Option<ModelSelection>,
   }
   
   struct ModelSelection {
       provider: String,
       model_id: String,
       name: String,
   }
   ```
2. Add model dropdown to tab header:
   - Show **spinner** while AuthSyncState is InProgress
   - After sync complete: show current model name (or default)
   - Dropdown populated from curated models list
   - "Manage Models..." option opens settings
   - If sync failed: show error icon with tooltip
3. Update prompt sending logic:
   - Include model in API request via ServerClient
   - Request format: `POST /session/{id}/message` with model field
   - Fall back to default model if no tab-specific selection

### Phase 6: Testing & Polish 🔲 OPTIONAL (Not Implemented)
1. Test async key sync:
   - Verify keys sent to server on startup
   - Check spinner appears in model dropdown
   - Confirm models populate after sync
   - Test offline behavior (server not running)
2. Test model discovery:
   - Call GET /config/providers
   - Verify all synced providers appear
   - Check model metadata is correct
3. Test error handling:
   - Missing .env file (should show helpful error)
   - Invalid/expired API keys (server returns 401)
   - Network failures (retry logic)
   - Server offline on startup (graceful degradation)
4. Test model switching:
   - Switch models mid-conversation
   - Verify correct model used in API requests
5. Add UI polish:
   - Spinner in model dropdown during auth sync
   - Success checkmark after sync completes
   - Error toasts for sync/API failures
   - Tooltips showing model details (context window, capabilities)
   - Provider icons/badges
   - Status indicator showing sync progress

## Server Provider Response

The server's `GET /config/providers` endpoint returns a unified format, abstracting away provider-specific differences:

```json
{
  "providers": [
    {
      "id": "openai",
      "name": "OpenAI",
      "source": "env",
      "models": {
        "gpt-4": {
          "id": "gpt-4",
          "name": "GPT-4",
          "providerID": "openai",
          "capabilities": {
            "temperature": true,
            "reasoning": false,
            "attachment": true,
            "toolcall": true,
            "input": {
              "text": true,
              "audio": false,
              "image": true,
              "video": false,
              "pdf": false
            },
            "output": {
              "text": true,
              "audio": false,
              "image": false,
              "video": false,
              "pdf": false
            }
          },
          "cost": {
            "input": 0.00003,
            "output": 0.00006,
            "cache": {
              "read": 0.000015,
              "write": 0.0000375
            }
          },
          "limit": {
            "context": 128000,
            "output": 4096
          }
        },
        "gpt-4-turbo": { /* ... */ }
      }
    },
    {
      "id": "anthropic",
      "name": "Anthropic",
      "source": "env",
      "models": {
        "claude-3-5-sonnet-20241022": { /* ... */ }
      }
    }
  ],
  "default": {
    "openai": "gpt-4",
    "anthropic": "claude-3-5-sonnet-20241022"
  }
}
```

EGUI doesn't need to handle provider-specific response formats—the server does all the heavy lifting.

## Error Handling

### Configuration Errors
- Missing config/models.toml → create default with empty curated list
- Invalid TOML syntax → show error dialog, don't load config
- Missing .env file → show setup prompt: "Add API keys to .env file"

### Startup Sync Errors
- Server offline → show error icon in model dropdown, allow retry
- Network failure → retry with exponential backoff, show status
- Invalid API key → server returns error, show which provider failed
- Partial sync success → show which providers succeeded/failed

### Runtime Errors
- Model discovery fails → show error toast, allow retry
- Server returns empty provider list → show "No providers configured" message
- Invalid model selection → fall back to default model
- Prompt fails due to missing credentials → show "Provider not configured" error

## Security Considerations

1. **API Key Storage:**
   - Store in `.env` file adjacent to executable (EGUI side)
   - Add `.env` to `.gitignore` (never commit)
   - Ship `.env.example` with placeholders in production builds
   - Keys synced to server's `~/.local/share/opencode/auth.json` (with 0600 permissions)
   - Keys never logged or exposed in UI

2. **Key Transmission:**
   - Keys sent over HTTPS to server's `PUT /auth/:id` endpoint
   - If server is localhost, ensure TLS or warn user
   - Keys transmitted only once per startup (unless changed)

3. **Server-Side Security:**
   - Server stores keys in auth.json with restrictive file permissions
   - Server validates API keys before accepting them
   - Server handles all provider authentication

4. **Config File Integrity:**
   - Validate config/models.toml schema at load time
   - Sanitize user input in model names
   - Backup config before writing changes

## Future Enhancements (Out of Scope)

- Model performance tracking per task type
- Automatic model recommendation based on prompt
- Cost tracking and budgets per model
- Model capability matrix view
- Batch model testing (send same prompt to multiple models)
- Model aliases/nicknames
- Favorite models (quick access)
- Recently used models list
- Model usage statistics
