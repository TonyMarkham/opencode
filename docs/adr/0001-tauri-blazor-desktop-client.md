# ADR-0001: Add Tauri + Blazor WebAssembly Desktop Client

**Status:** Proposed

**Date:** 2026-01-02

**Deciders:** Tony (repository owner)

**Context Owners:** Desktop client development team

---

## Context

The OpenCode project currently has a native Rust desktop client built with egui (`clients/egui/`). The egui client provides:

- Auto server discovery and spawning
- Multi-session tabs with real-time streaming
- Markdown rendering with tool call visualization
- Speech-to-text (Whisper-based, push-to-talk)
- OAuth authentication for Anthropic
- Configurable UI settings

There is also an existing Tauri infrastructure in the repository (`packages/tauri/`) that currently uses a SolidJS frontend and wraps the `@opencode-ai/desktop` package.

### Current State

1. **egui client** (`clients/egui/`):
   - Pure Rust implementation using immediate-mode GUI
   - Depends on local egui fork with custom patches for sleep/wake fixes
   - Includes custom audio pipeline (cpal + whisper-rs)
   - Direct HTTP communication with OpenCode server
   - Binary size and performance optimized

2. **Existing Tauri package** (`packages/tauri/`):
   - Tauri v2 with SolidJS frontend
   - Already integrated into monorepo build system
   - Uses `@opencode-ai/desktop` shared package
   - Has working Tauri plugin ecosystem (dialog, shell, updater, opener)

### Requirements Driving This Decision

1. **Alternative UI technology**: Desire to explore .NET/Blazor ecosystem as an alternative to Rust-only egui
2. **Web technology benefits**: Easier styling, component libraries (e.g., Radzen), and potentially faster UI iteration
3. **Cross-platform consistency**: Blazor WASM provides consistent rendering across platforms
4. **Developer experience**: .NET developers may find Blazor more accessible than Rust/egui
5. **Shared infrastructure**: Can leverage existing Tauri v2 setup and Rust backend code
6. **CRITICAL: No custom JavaScript**: All JavaScript must be machine-generated (by Blazor compiler). Zero hand-written JS to maintain

### Constraints

1. **Monorepo structure**: Must fit into existing Bun-based monorepo with TypeScript/Rust components
2. **Build complexity**: Adding .NET SDK requirement increases build dependencies
3. **Feature parity expectation**: Users may expect similar functionality to egui client
4. **Maintenance burden**: Another client means more code to maintain
5. **Existing Tauri package**: The repo already has `packages/tauri/` with SolidJS - need to decide on coexistence
6. **ABSOLUTE CONSTRAINT: Zero custom JavaScript**
   - No hand-written `.js` files to maintain
   - Only Blazor-generated JS allowed (e.g., `_framework/blazor.webassembly.js`)
   - All Tauri IPC must be called from C# using Blazor's JSInterop
   - No JavaScript helpers, no custom event handlers, no manual DOM manipulation
   - This is a **hard requirement** - any solution requiring custom JS is rejected

### Technical Feasibility

Research confirms Tauri + Blazor WASM is a proven combination:

- Official Blazor template in `create-tauri-app`
- Multiple production examples (tauri-blazor-radzen-boilerplate, TauriWithBlazor)
- Tauri v2 supports Blazor WASM with standard IPC bridge via `@tauri-apps/api`
- Blazor component libraries (Radzen, MudBlazor) work in Tauri context

## Decision

**We will create a new Tauri + Blazor WebAssembly desktop client as an alternative to the egui client**, with the following approach:

1. **Create new client directory**: `clients/tauri-blazor/` (separate from `packages/tauri/`)
2. **Blazor hosting model**: Use Blazor WebAssembly (client-side) not Blazor Hybrid/Server
3. **Feature scope**: Start with core features (session management, chat streaming, markdown rendering) and incrementally add advanced features
4. **Shared Rust code**: Leverage Tauri commands to share server discovery and API client logic with the egui implementation
5. **Coexistence strategy**: Keep both clients maintained, allowing users to choose based on preference

### Key Technical Decisions

- **Frontend**: Blazor WebAssembly 8.0+ (.NET 8+)
- **Desktop framework**: Tauri v2 (Rust backend)
- **UI components**: Radzen Blazor components (based on research examples)
- **Server communication**:
  - Tauri commands for server discovery/process management
  - Direct HTTP from Blazor for API calls (similar to egui)
- **Build integration**: Separate build from existing `packages/tauri`, no conflict with SolidJS version
- **JavaScript policy**:
  - **ZERO custom JavaScript files**
  - All Tauri IPC via C# `IJSRuntime.InvokeAsync` calling Blazor-generated JS
  - Blazor handles all DOM interaction natively
  - No `wwwroot/js/` directory with custom scripts

## Alternatives Considered

### Alternative 1: Enhance Existing packages/tauri with Blazor

**Description:** Replace the SolidJS frontend in `packages/tauri/` with Blazor instead of creating a new client

**Pros:**

- Single Tauri client to maintain
- Reuse existing build configuration
- Less duplication

**Cons:**

- Breaks existing SolidJS implementation that may be in use
- Conflates two different frontend technologies in same package
- May confuse existing users of `packages/tauri`

**Why rejected:** The existing `packages/tauri` appears to be a separate effort with its own `@opencode-ai/desktop` abstraction. Creating a clean separate client allows both to coexist and serves different user preferences.

### Alternative 2: Avalonia UI or other .NET-native framework

**Description:** Use Avalonia UI, MAUI, or WPF instead of Blazor for .NET-based desktop UI

**Pros:**

- True native .NET desktop framework
- No need for Tauri (pure .NET)
- Potentially better performance than WebView-based approach

**Cons:**

- No existing Tauri integration (loses server discovery code reuse)
- Steeper learning curve for web developers
- Less component library ecosystem compared to Blazor
- MAUI has cross-platform issues, WPF is Windows-only
- Avalonia is less mature than Blazor

**Why rejected:** Blazor + Tauri leverages existing Tauri infrastructure and Rust backend code, while providing web-like developer experience. Avalonia would require reimplementing all platform-specific code.

### Alternative 3: Electron + Blazor

**Description:** Use Electron instead of Tauri as the desktop shell for Blazor WASM

**Pros:**

- Larger ecosystem and community
- More mature tooling
- Well-known to JavaScript developers

**Cons:**

- Significantly larger bundle size (100+ MB vs <10 MB for Tauri)
- Higher memory footprint
- Doesn't leverage existing Rust code in the repository
- OpenCode philosophy appears to favor Rust/performance-focused tools

**Why rejected:** Tauri aligns better with the existing codebase's Rust foundation and provides better performance characteristics. The repo already has Tauri v2 setup.

### Alternative 4: Keep egui-only, no alternative client

**Description:** Focus all efforts on improving the egui client instead of adding alternatives

**Pros:**

- Single client to maintain
- No split in development effort
- Simpler for users (one choice)
- egui is performant and lightweight

**Cons:**

- Limits accessibility for .NET/web developers
- egui's immediate-mode paradigm can be harder to customize
- Current egui implementation requires local forks (maintenance burden)
- Misses opportunity to explore web tech benefits for desktop

**Why rejected:** Having alternatives allows the project to serve different developer communities and evaluate different UI paradigms. The Tauri+Blazor client can validate whether web technologies offer meaningful benefits for this use case without abandoning the egui client.

## Consequences

### Positive

- **Broader developer appeal**: .NET/C# developers can contribute to desktop client
- **Rich component ecosystem**: Access to mature Blazor component libraries (Radzen, MudBlazor, etc.)
- **Rapid UI iteration**: Web technologies generally allow faster UI experimentation
- **Shared Rust backend**: Can reuse server discovery, process management via Tauri commands
- **Learning opportunity**: Validates web tech approach vs native Rust for this application
- **User choice**: Different users can choose client based on preference (web-like vs native feel)
- **No JavaScript maintenance**: Blazor handles all JS generation - zero custom JS to maintain
- **Type safety**: C# for UI logic instead of JavaScript/TypeScript
- **Single language stack**: C# in frontend, Rust in backend - no JS layer

### Negative

- **Increased maintenance burden**: Two desktop clients to maintain and keep in sync
- **Build complexity**: Requires .NET SDK in addition to Rust toolchain
- **Larger bundle size**: Blazor WASM + Tauri will be larger than pure egui (but smaller than Electron)
- **Performance characteristics**: WebView-based rendering may be less performant than native egui
- **Feature parity challenge**: Risk of features diverging between clients
- **Documentation burden**: Need to document two different client options
- **Blazor WASM limitations**: Some advanced features may require JSInterop (but still no custom JS allowed)
- **.NET runtime overhead**: Mono WASM runtime adds size and startup time compared to pure JS frameworks

### Neutral

- **Different look and feel**: Tauri+Blazor will have web-like UI vs egui's native feel
- **Packaging differences**: Blazor client will bundle .NET runtime, different updater mechanisms
- **Testing requirements**: Need to test both clients across platforms

## Implementation Notes

### Lessons from Cognexus

The repository owner has a working Tauri + Blazor application (Cognexus) that demonstrates this stack successfully. Key insights:

1. **.NET 10.0 compatibility**: Cognexus uses `.NET 10.0` (net10.0) with Blazor WASM - very cutting edge
2. **Tauri 2.9.5**: Uses latest Tauri 2.x with features: `devtools`, `macos-private-api`
3. **Build workflow**: Uses `justfile` for orchestrating complex builds (WASM components → Blazor publish → Tauri build)
4. **Project structure**: Frontend in separate directory (`frontend/cognexus/`), Tauri app in `apps/desktop/cognexus/`
5. **Tauri commands**: Clean separation with `#[tauri::command]` functions in main.rs, managed state via `.manage()`
6. **Blazor publish target**: Sets `<PublishDir>` to output to Tauri's frontend directory
7. **tauri.conf.json**: Points `frontendDist` to `./frontend/wwwroot` (Blazor WASM output)

**Direct applicability to OpenCode:**

- Same Rust workspace pattern (multiple backend crates + Tauri app)
- Proven .NET 10 + Tauri 2.9.5 compatibility
- justfile build orchestration works well
- Can reuse Tauri state management pattern for server discovery

### Phase 1: Project Scaffold (Week 1)

1. Create `clients/tauri-blazor/` directory structure (mirroring Cognexus pattern):

   ```
   clients/tauri-blazor/
   ├── src-tauri/              # Rust backend (Tauri)
   │   ├── src/
   │   │   ├── main.rs         # Tauri commands + state management
   │   │   ├── commands/       # Tauri command handlers
   │   │   │   ├── mod.rs
   │   │   │   ├── server.rs   # Server discovery/spawn/health
   │   │   │   ├── auth.rs     # OAuth and API key sync
   │   │   │   └── session.rs  # Session management
   │   │   └── state.rs        # Shared Tauri state
   │   ├── Cargo.toml
   │   ├── build.rs
   │   └── tauri.conf.json
   ├── frontend/               # .NET Blazor WASM project
   │   ├── Pages/
   │   │   ├── Chat.razor      # Main chat interface
   │   │   ├── Settings.razor  # Settings panel
   │   │   └── Home.razor
   │   ├── Components/
   │   │   ├── MessageList.razor
   │   │   ├── InputBox.razor
   │   │   ├── SessionTabs.razor
   │   │   └── ToolCallView.razor
   │   ├── Services/
   │   │   ├── IServerService.cs      # Interface for server ops
   │   │   ├── ServerService.cs       # Calls Tauri commands
   │   │   ├── ISessionService.cs
   │   │   └── SessionService.cs
   │   ├── wwwroot/
   │   │   ├── css/
   │   │   └── js/
   │   ├── Program.cs
   │   ├── App.razor
   │   ├── _Imports.razor
   │   └── OpenCodeBlazor.csproj      # .NET 9+ with PublishDir set
   ├── justfile                # Build orchestration (like Cognexus)
   └── README.md
   ```

2. **Blazor project setup:**

   ```xml
   <!-- OpenCodeBlazor.csproj -->
   <Project Sdk="Microsoft.NET.Sdk.BlazorWebAssembly">
     <PropertyGroup>
       <TargetFramework>net9.0</TargetFramework>  <!-- or net10.0 -->
       <Nullable>enable</Nullable>
       <ImplicitUsings>enable</ImplicitUsings>
     </PropertyGroup>

     <ItemGroup>
       <PackageReference Include="Microsoft.AspNetCore.Components.WebAssembly" Version="9.0.0" />
       <PackageReference Include="Microsoft.AspNetCore.Components.WebAssembly.DevServer" Version="9.0.0" />
       <PackageReference Include="Radzen.Blazor" Version="5.0.0" />  <!-- UI components -->
       <PackageReference Include="Markdig" Version="0.37.0" />        <!-- Markdown rendering -->
     </ItemGroup>

     <PropertyGroup>
       <PublishDir>../src-tauri/frontend/</PublishDir>
     </PropertyGroup>
   </Project>
   ```

3. **Tauri app setup:**

   ```rust
   // main.rs
   use tauri::Manager;

   mod commands;
   mod state;

   use state::AppState;

   fn main() {
       tauri::Builder::default()
           .invoke_handler(tauri::generate_handler![
               commands::server::discover_server,
               commands::server::spawn_server,
               commands::server::check_health,
               commands::auth::sync_auth_token,
               commands::session::create_session,
               commands::session::send_message,
           ])
           .setup(|app| {
               // Initialize shared state
               let state = AppState::new();
               app.manage(state);
               Ok(())
           })
           .run(tauri::generate_context!())
           .expect("error while running tauri application");
   }
   ```

4. **tauri.conf.json** (following Cognexus pattern):

   ```json
   {
     "$schema": "https://schema.tauri.app/config/2",
     "productName": "OpenCode",
     "version": "0.0.1",
     "identifier": "com.opencode.tauri-blazor",
     "build": {
       "frontendDist": "./frontend/wwwroot"
     },
     "app": {
       "withGlobalTauri": true,
       "windows": [
         {
           "title": "OpenCode",
           "width": 1200,
           "height": 800
         }
       ]
     },
     "bundle": {
       "active": true,
       "targets": "all"
     }
   }
   ```

5. **justfile** for build orchestration:

   ```makefile
   # Build Blazor frontend
   build-frontend:
       cd frontend && dotnet publish -c Release

   # Development mode
   dev: build-frontend
       cd src-tauri && cargo tauri dev

   # Production build
   build: build-frontend
       cd src-tauri && cargo tauri build
   ```

6. Add to monorepo tooling (turbo.json, root package.json if needed)

### Phase 2: Core Features (Weeks 2-4)

**Server Discovery & Communication (Week 2)**

1. **Extract shared Rust code** from `clients/egui/src/discovery/` into workspace crate:

   ```
   crates/opencode-client-core/
   ├── src/
   │   ├── discovery/
   │   │   ├── mod.rs
   │   │   ├── process.rs   # From egui
   │   │   └── spawn.rs     # From egui
   │   └── lib.rs
   └── Cargo.toml
   ```

2. **Implement Tauri commands** in `clients/tauri-blazor/src-tauri/src/commands/server.rs`:

   ```rust
   use opencode_client_core::discovery::{discover, spawn_and_wait, check_health};

   #[tauri::command]
   pub async fn discover_server() -> Result<Option<ServerInfo>, String> {
       discover().await.map_err(|e| e.to_string())
   }

   #[tauri::command]
   pub async fn spawn_server() -> Result<ServerInfo, String> {
       spawn_and_wait().await.map_err(|e| e.to_string())
   }

   #[tauri::command]
   pub async fn check_health(port: u16) -> Result<bool, String> {
       check_health(port).await.map_err(|e| e.to_string())
   }
   ```

3. **Create Blazor service** for server interaction (`ServerService.cs`):

   ```csharp
   using Microsoft.JSInterop;

   public class ServerService : IServerService
   {
       private readonly IJSRuntime _jsRuntime;

       public async Task<ServerInfo?> DiscoverServerAsync()
       {
           return await _jsRuntime.InvokeAsync<ServerInfo?>(
               "window.__TAURI__.invoke",
               "discover_server"
           );
       }

       public async Task<ServerInfo> SpawnServerAsync()
       {
           return await _jsRuntime.InvokeAsync<ServerInfo>(
               "window.__TAURI__.invoke",
               "spawn_server"
           );
       }
   }
   ```

**Chat UI (Week 3)**

1. Build main chat page with Radzen components:
   - `MessageList.razor` - Displays conversation history
   - `InputBox.razor` - Message input with send button
   - `SessionTabs.razor` - Multi-session tab management

2. Integrate markdown rendering using Markdig (C# markdown library):
   ```csharp
   @code {
       private string RenderMarkdown(string markdown)
       {
           var pipeline = new MarkdownPipelineBuilder()
               .UseAdvancedExtensions()
               .Build();
           return Markdown.ToHtml(markdown, pipeline);
       }
   }
   ```

**Streaming Messages (Week 4)**

1. Implement Server-Sent Events (SSE) client in Blazor for streaming responses
2. Or use Tauri event system for streaming from Rust backend
3. Update UI reactively as message chunks arrive

### Phase 3: Advanced Features (Weeks 5-8)

1. OAuth authentication flow (reuse egui auth patterns)
2. Model selection and provider management
3. Tab management for multi-session support
4. Tool call visualization

### Phase 4: Polish and Optional Features (Weeks 9-12)

1. Audio/STT integration (evaluate web-based vs Tauri command approach)
2. Keyboard shortcuts and accessibility
3. Updater integration
4. Cross-platform testing and packaging

### Integration Points

- **Shared Rust code**: Extract server discovery, API client, and OAuth into shared crate that both egui and tauri-blazor can use
- **Tauri commands**: Define clean IPC boundary for operations like:
  - `discover_server()` → `Option<ServerInfo>`
  - `spawn_server()` → `Result<(), String>`
  - `check_health(port)` → `bool`
  - `sync_auth(provider, token)` → `Result<(), String>`

### Migration Considerations

- No migration needed - this is a new alternative, not a replacement
- Users can try the Blazor client without affecting their egui client usage
- Configuration files should be compatible between clients where possible

### Testing Strategy

1. **Unit tests**: Rust Tauri commands, Blazor component tests
2. **Integration tests**: Server discovery, authentication flows
3. **Manual testing**: Cross-platform (macOS, Windows, Linux) UI/UX validation
4. **Performance benchmarks**: Compare startup time, memory usage vs egui baseline

### Rollout Approach

1. **Alpha release**: Internal testing with core features only
2. **Beta release**: Invite community feedback on GitHub discussions
3. **Stable release**: Mark as production-ready in documentation
4. **Documentation**: Add "Choosing a Desktop Client" guide comparing egui vs tauri-blazor

### Success Criteria

- Parity with egui core features (chat, streaming, sessions, auth)
- Acceptable performance (<2s startup, smooth streaming)
- Positive community feedback from .NET developers
- Maintainable codebase with shared Rust logic

### Open Questions

1. ~~Should we extract shared Rust code into `crates/opencode-client-core/`?~~ **YES** - Cognexus demonstrates this pattern works well
2. What's the minimum .NET version to target? (.NET 8 LTS, .NET 9, or .NET 10 like Cognexus?)
3. Should the Blazor client support mobile (via Tauri mobile) in future?
4. How to handle audio/STT - web APIs, Tauri command wrapper around whisper-rs, or skip initially?
5. Should we use `justfile` (like Cognexus) or integrate into existing Bun-based build system?

### Technical Validation from Cognexus

The repository owner's Cognexus project provides concrete proof that Tauri 2.9.5 + Blazor WASM + .NET 10 works well together:

**Proven working stack:**

- Tauri 2.9.5 with features: `devtools`, `macos-private-api`
- .NET 10.0 Blazor WebAssembly
- Rust workspace with multiple backend crates
- `justfile` for build orchestration
- Protobuf for Rust ↔ Blazor communication (optional for OpenCode)

**Build workflow that works:**

1. Publish Blazor WASM to `src-tauri/frontend/` directory
2. Configure `tauri.conf.json` with `"frontendDist": "./frontend/wwwroot"`
3. Tauri bundles the published Blazor WASM output
4. Tauri commands expose Rust functionality to Blazor via IPC

**Relevant patterns to reuse:**

- Tauri state management with `.manage()` for shared backend state
- Blazor services that wrap `window.__TAURI__.invoke()` calls **via C# IJSRuntime only**
- Workspace dependencies pattern for sharing code between crates
- `justfile` tasks for complex multi-step builds

**CRITICAL DIFFERENCE from Cognexus:**

- Cognexus has `wwwroot/js/renderer-helper.js` - **we will NOT have this**
- All Tauri interaction must go through C# → IJSRuntime → Blazor-generated JS
- No `wwwroot/js/` directory with custom JavaScript
- Blazor component libraries (Radzen) handle all DOM interaction internally

## References

- [create-tauri-app Blazor template](https://github.com/tauri-apps/create-tauri-app) - Official Tauri scaffolding tool includes Blazor template
- [tauri-blazor-radzen-boilerplate](https://github.com/itsalfredakku/tauri-blazor-radzen-boilerplate) - Production example of Tauri + Blazor + Radzen
- [TauriWithBlazor](https://github.com/rodiniz/TauriWithBlazor) - Minimal Tauri + Blazor template
- [Stack Overflow: Invoking Tauri API in Blazor WASM](https://stackoverflow.com/questions/75359781/how-to-invoke-tauri-api-in-blazor-wasm) - Technical guidance on Blazor/Tauri IPC
- [Tauri v2 Documentation](https://v2.tauri.app) - Official Tauri framework docs
- [Radzen Blazor Components](https://blazor.radzen.com/) - UI component library commonly used with Tauri+Blazor
