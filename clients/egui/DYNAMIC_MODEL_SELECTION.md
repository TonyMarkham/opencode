# Dynamic Model Selection for OpenCode EGUI

Status: **Planning** (2025-12-08)

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

**Hybrid Architecture** - EGUI manages API keys locally and syncs them to the server. Server handles all provider logic and model inference.

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
2. EGUI calls `GET /config/providers` to fetch available models from server
3. Server returns list of all models from configured providers (using stored API keys)
4. User searches/filters model list in EGUI UI
5. User selects model → added to curated list in `config/models.toml`
6. Curated list saved locally (EGUI only)

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

### config/models.toml (Curated Models + UI Preferences)

Stored in `config/` directory adjacent to the EGUI executable. This file contains **only UI state**, not provider configurations (server handles that).

```toml
[models]
default_model = "openai/gpt-4"           # Default for new tabs

# Curated models list (user's favorites)
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
```

**Note:** Model metadata (description, capabilities, context window) comes from the server's provider database, not from this file. This keeps the client config lightweight.

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

### Provider Endpoints (Existing)

#### GET /config/providers
Returns all available models from configured providers.

**Response:**
```json
{
  "providers": [
    {
      "id": "openai",
      "name": "OpenAI",
      "models": {
        "gpt-4": {
          "id": "gpt-4",
          "name": "GPT-4",
          "capabilities": {
            "vision": true,
            "toolcall": true
          },
          "limit": {
            "context": 128000,
            "output": 4096
          }
        }
      }
    }
  ],
  "default": {
    "openai": "gpt-4",
    "anthropic": "claude-3-5-sonnet-20241022"
  }
}
```

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

### Phase 1: Config Infrastructure & Build System ⏳ TODO
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

### Phase 2: Async API Key Sync ⏳ TODO
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

### Phase 3: Server API Client ⏳ TODO
1. Create `src/client/server.rs`:
   ```rust
   pub struct ServerClient {
       base_url: String,
       client: reqwest::Client,
   }
   
   impl ServerClient {
       pub async fn sync_auth(&self, provider: &str, key: &str) -> Result<()> {
           // PUT /auth/{provider}
       }
       
       pub async fn get_providers(&self) -> Result<ProvidersResponse> {
           // GET /config/providers
       }
       
       pub async fn send_message(&self, session_id: &str, request: MessageRequest) -> Result<()> {
           // POST /session/{id}/message
       }
   }
   ```
2. Create `src/types/models.rs`:
   ```rust
   struct CuratedModel {
       name: String,
       provider: String,
       model_id: String,
   }
   
   struct ProviderInfo {
       id: String,
       name: String,
       models: HashMap<String, ModelInfo>,
   }
   
   struct ModelInfo {
       id: String,
       name: String,
       capabilities: ModelCapabilities,
       limit: ModelLimits,
   }
   ```
3. Create `src/config/models.rs`:
   - Load config/models.toml at startup
   - Load curated models list
   - Save updated curated list back to TOML

### Phase 4: Model Management UI ⏳ TODO
1. Create ModelDiscoveryWindow:
   - Step 1: Call `GET /config/providers` to fetch available models
   - Step 2: Show provider list with model counts
   - Step 3: When provider selected, show searchable/filterable model list
   - Step 4: Add selected model(s) to curated list in config/models.toml
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

### Phase 5: Per-Tab Model Selection ⏳ TODO
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

### Phase 6: Testing & Polish ⏳ TODO
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
