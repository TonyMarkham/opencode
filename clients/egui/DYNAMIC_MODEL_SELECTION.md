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

### Component Structure

```
Server (OpenCode):
  .env                    - API keys and discovery endpoints
  src/api/models.ts       - Model discovery and caching
  src/config/models.ts    - Curated model list management
  
EGUI Client:
  src/config.rs           - Per-tab model preferences
  src/ui/settings.rs      - Model discovery UI
  src/ui/tab.rs          - Model dropdown in tab header
  src/client/api.rs       - Model fetching from server
```

### Data Flow

**Adding a Model:**
1. User opens Settings → "Add Model"
2. Selects provider (OpenAI, Anthropic, Google, OpenRouter)
3. Server fetches available models from provider API
4. User searches/filters model list
5. User selects model → added to curated list
6. Curated list saved to server config
7. EGUI client refreshes available models

**Using a Model:**
1. User clicks model dropdown in tab header
2. Shows curated model list
3. User selects model
4. Tab stores selected model preference
5. Next prompt includes model in `SessionPromptParams.model`
6. Server uses specified model for that conversation

## Configuration

### Server (.env)

```bash
# API Keys
OPENAI_API_KEY=sk-...
ANTHROPIC_API_KEY=sk-ant-...
GOOGLE_API_KEY=...
OPENROUTER_API_KEY=...

# Model Discovery Endpoints
OPENAI_MODELS_URL=https://api.openai.com/v1/models
ANTHROPIC_MODELS_URL=https://api.anthropic.com/v1/models
GOOGLE_MODELS_URL=https://generativelanguage.googleapis.com/v1/models
OPENROUTER_MODELS_URL=https://openrouter.ai/api/v1/models
```

### Server (opencode.json)

```json
{
  "models": {
    "curated": [
      {
        "name": "GPT-5.1",
        "provider": "openai",
        "modelId": "gpt-5.1",
        "description": "Latest GPT model with enhanced reasoning",
        "contextWindow": 128000,
        "supportsVision": true,
        "supportsTools": true
      },
      {
        "name": "Claude 4.5 Sonnet",
        "provider": "anthropic",
        "modelId": "claude-sonnet-4-5-20250929",
        "description": "Best for complex reasoning and analysis",
        "contextWindow": 200000,
        "supportsVision": true,
        "supportsTools": true
      },
      {
        "name": "Gemini 3 Pro",
        "provider": "google",
        "modelId": "gemini-3-pro",
        "description": "Google's latest multimodal model",
        "contextWindow": 1000000,
        "supportsVision": true,
        "supportsTools": true
      }
    ]
  }
}
```

### EGUI Client (config.toml)

```toml
[models]
# Default model for new tabs
default_model = "openai/gpt-5.1"

# Per-tab model preferences (stored at runtime, persisted per session)
# Format: tab_index = "provider/modelId"
```

## API Endpoints

### Server Endpoints

#### GET /api/models
Returns the curated model list.

**Response:**
```json
{
  "models": [
    {
      "name": "GPT-5.1",
      "provider": "openai",
      "modelId": "gpt-5.1",
      "description": "Latest GPT model",
      "contextWindow": 128000,
      "supportsVision": true,
      "supportsTools": true
    }
  ]
}
```

#### GET /api/models/discover?provider=openai
Fetches available models from specified provider's API.

**Query Parameters:**
- `provider`: One of `openai`, `anthropic`, `google`, `openrouter`

**Response:**
```json
{
  "provider": "openai",
  "models": [
    {
      "id": "gpt-5.1",
      "name": "GPT-5.1",
      "description": "Latest model",
      "contextWindow": 128000,
      "capabilities": {
        "vision": true,
        "tools": true,
        "streaming": true
      }
    }
  ]
}
```

#### POST /api/models/curated
Adds a model to the curated list.

**Request Body:**
```json
{
  "provider": "openai",
  "modelId": "gpt-5.1",
  "name": "GPT-5.1" // Optional custom name
}
```

#### DELETE /api/models/curated/:provider/:modelId
Removes a model from the curated list.

### Existing Endpoint Usage

Sessions already support per-prompt model selection via `POST /session/{id}/message`:

```json
{
  "parts": [...],
  "model": {
    "providerID": "openai",
    "modelID": "gpt-5.1"
  }
}
```

No session-level model storage needed - model is specified per prompt.

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

### Phase 1: Server - Model Discovery ⏳ TODO
1. Add `.env.example` with provider endpoints
2. Create `/api/models` endpoints:
   - GET /api/models - return curated list
   - GET /api/models/discover?provider=X - fetch from provider API
   - POST /api/models/curated - add to curated list
   - DELETE /api/models/curated/:provider/:modelId - remove from list
3. Implement model fetching for each provider:
   - OpenAI: `/v1/models`
   - Anthropic: `/v1/models`
   - Google: `/v1/models`
   - OpenRouter: `/api/v1/models`
4. Add caching layer (5 minute TTL) to avoid rate limits
5. Store curated list in `opencode.json`

### Phase 2: EGUI Client - Model Management UI ⏳ TODO
1. Add models config section to AppConfig
2. Create ModelDiscoveryWindow:
   - Provider selection screen
   - Model list with search filter
   - Add/remove model actions
3. Create ModelManager:
   - Fetch curated models from server
   - Cache locally
   - Update on settings save
4. Add "Models" section to Settings window:
   - Show curated models list
   - "Add Model" button opens ModelDiscoveryWindow
   - Default model selector

### Phase 3: EGUI Client - Per-Tab Model Selection ⏳ TODO
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
   - Show current model name
   - Dropdown with curated models list
   - "Manage Models..." option opens settings
3. Update prompt sending logic:
   - Include model in API request when set
   - Use default model if no tab-specific model selected
4. Persist tab model preferences (optional):
   - Store in client config
   - Restore on app restart per session

### Phase 4: Testing & Polish ⏳ TODO
1. Test model discovery for all providers
2. Test model switching mid-conversation
3. Handle API errors gracefully:
   - Invalid API keys
   - Rate limiting
   - Network failures
4. Add loading indicators:
   - Fetching models from provider
   - Updating curated list
5. Add tooltips/help text:
   - Model descriptions
   - Context window info
   - Capabilities badges

## Provider-Specific Implementation Notes

### OpenAI
- Endpoint: `GET https://api.openai.com/v1/models`
- Auth: `Authorization: Bearer $OPENAI_API_KEY`
- Returns: List of model objects with id, created, owned_by

### Anthropic
- Endpoint: `GET https://api.anthropic.com/v1/models`
- Auth: `x-api-key: $ANTHROPIC_API_KEY`
- Returns: List of models with capabilities

### Google (Gemini)
- Endpoint: `GET https://generativelanguage.googleapis.com/v1/models`
- Auth: `?key=$GOOGLE_API_KEY` (query param)
- Returns: List of models with descriptions and capabilities

### OpenRouter
- Endpoint: `GET https://openrouter.ai/api/v1/models`
- Auth: `Authorization: Bearer $OPENROUTER_API_KEY`
- Returns: Comprehensive list with pricing, context windows, capabilities
- Note: 200+ models, search is essential

## Error Handling

### Server-Side
- Missing API key → return 401 with helpful message
- Invalid provider → return 400 with valid provider list
- Provider API error → cache last successful response, return cached data
- Rate limiting → implement exponential backoff, show retry time to user

### Client-Side
- Network error → show error toast, allow retry
- Empty curated list → prompt user to add models
- Model no longer available → mark as deprecated, suggest alternatives
- Invalid model selection → fall back to default model

## Security Considerations

1. **API Key Storage:**
   - Store in `.env` file (server-side only)
   - Never expose keys to EGUI client
   - Use environment variables, not config files

2. **Model List Validation:**
   - Validate provider responses before caching
   - Sanitize model names/descriptions for display
   - Limit model list size to prevent DoS

3. **Rate Limiting:**
   - Cache provider responses (5 min TTL)
   - Limit discovery requests per user session
   - Implement request throttling

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
