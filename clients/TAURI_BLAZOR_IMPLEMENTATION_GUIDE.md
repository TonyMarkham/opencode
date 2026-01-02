# Tauri + Blazor Client Implementation Guide

**Status:** Draft  
**Related ADR:** [ADR-0001: Add Tauri + Blazor WebAssembly Desktop Client](../docs/adr/0001-tauri-blazor-desktop-client.md)

This guide provides step-by-step instructions for implementing the Tauri + Blazor desktop client, based on proven patterns from the Cognexus reference implementation.

## Prerequisites

Before starting, ensure you have:

- **Rust** toolchain (latest stable via rustup)
- **.NET SDK** 9.0+ or 10.0+ ([download](https://dotnet.microsoft.com/download))
- **Node.js** (for Tauri CLI)
- **Tauri CLI**: `cargo install tauri-cli@^2.0.0`
- **Just** (optional but recommended): `cargo install just` or `brew install just`
- **Bun** (already required for OpenCode monorepo)

## CRITICAL CONSTRAINT: No Custom JavaScript

**This implementation has a HARD REQUIREMENT: ZERO custom JavaScript.**

- ❌ No `wwwroot/js/` directory with custom scripts
- ❌ No hand-written `.js` files anywhere
- ❌ No JavaScript helpers or utilities
- ✅ ONLY Blazor-generated JavaScript (e.g., `_framework/blazor.webassembly.js`)
- ✅ All Tauri IPC via C# using `IJSRuntime.InvokeAsync()`
- ✅ All DOM interaction via Blazor components (Radzen, built-in)

**Why this matters:**

- Reduces maintenance burden - no JS codebase to manage
- Type safety - C# catches errors at compile time
- Single language for frontend logic - no context switching
- Blazor handles browser compatibility automatically

## Step 1: Create Project Structure

```bash
cd /Users/tony/git/opencode
mkdir -p clients/tauri-blazor/{src-tauri/src/commands,frontend}
```

### Directory Layout

```
clients/tauri-blazor/
├── src-tauri/                      # Rust backend (Tauri app)
│   ├── src/
│   │   ├── main.rs                 # Entry point
│   │   ├── commands/               # Tauri command handlers
│   │   │   ├── mod.rs
│   │   │   ├── server.rs           # Server discovery/spawn
│   │   │   ├── auth.rs             # OAuth & API keys
│   │   │   └── session.rs          # Session/chat management
│   │   └── state.rs                # Shared application state
│   ├── Cargo.toml
│   ├── build.rs
│   └── tauri.conf.json
├── frontend/                       # Blazor WASM project
│   ├── Pages/
│   ├── Components/
│   ├── Services/
│   ├── wwwroot/
│   ├── Program.cs
│   ├── App.razor
│   ├── _Imports.razor
│   └── OpenCodeBlazor.csproj
├── justfile                        # Build orchestration
└── README.md
```

## Step 2: Set Up Shared Rust Core

Extract server discovery and common code from egui client:

```bash
mkdir -p crates/opencode-client-core/src/discovery
```

### `crates/opencode-client-core/Cargo.toml`

```toml
[package]
name = "opencode-client-core"
version = "0.0.1"
edition = "2024"

[dependencies]
tokio = { version = "1.43", features = ["rt-multi-thread", "macros", "process"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
reqwest = { version = "0.12", features = ["json"] }
sysinfo = "0.30"
netstat2 = "0.9"
thiserror = "2.0"
```

### Extract Discovery Code

Copy these files from `clients/egui/src/discovery/`:

- `process.rs` → `crates/opencode-client-core/src/discovery/process.rs`
- `spawn.rs` → `crates/opencode-client-core/src/discovery/spawn.rs`

Update egui client to use the shared crate instead of local modules.

## Step 3: Initialize Tauri Project

### `clients/tauri-blazor/src-tauri/Cargo.toml`

```toml
[package]
name = "opencode-tauri-blazor"
version = "0.0.1"
edition = "2024"

[build-dependencies]
tauri-build = { version = "2", features = [] }

[dependencies]
tauri = { version = "2", features = ["devtools", "macos-private-api"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
tokio = { version = "1", features = ["full"] }
thiserror = "2"

# Shared OpenCode client code
opencode-client-core = { path = "../../../crates/opencode-client-core" }
```

### `clients/tauri-blazor/src-tauri/build.rs`

```rust
fn main() {
    tauri_build::build()
}
```

### `clients/tauri-blazor/src-tauri/tauri.conf.json`

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
        "height": 800,
        "resizable": true,
        "fullscreen": false
      }
    ],
    "security": {
      "csp": null
    }
  },
  "bundle": {
    "active": true,
    "targets": "all",
    "icon": ["icons/icon.png"]
  }
}
```

### `clients/tauri-blazor/src-tauri/src/main.rs`

```rust
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod state;

use tauri::Manager;
use state::AppState;

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            commands::server::discover_server,
            commands::server::spawn_server,
            commands::server::check_health,
            commands::server::stop_server,
        ])
        .setup(|app| {
            // Initialize shared application state
            let state = AppState::new();
            app.manage(state);
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

### `clients/tauri-blazor/src-tauri/src/state.rs`

```rust
use std::sync::Mutex;

#[derive(Default)]
pub struct AppState {
    pub server_port: Mutex<Option<u16>>,
    pub server_pid: Mutex<Option<u32>>,
}

impl AppState {
    pub fn new() -> Self {
        Self::default()
    }
}
```

### `clients/tauri-blazor/src-tauri/src/commands/mod.rs`

```rust
pub mod server;
```

### `clients/tauri-blazor/src-tauri/src/commands/server.rs`

```rust
use opencode_client_core::discovery::{ServerInfo, discover, spawn_and_wait, check_health};
use tauri::State;
use crate::state::AppState;

#[tauri::command]
pub async fn discover_server(
    state: State<'_, AppState>
) -> Result<Option<ServerInfo>, String> {
    match discover().await {
        Ok(Some(info)) => {
            // Store in state
            *state.server_port.lock().unwrap() = Some(info.port);
            if let Some(pid) = info.pid {
                *state.server_pid.lock().unwrap() = Some(pid);
            }
            Ok(Some(info))
        }
        Ok(None) => Ok(None),
        Err(e) => Err(e.to_string()),
    }
}

#[tauri::command]
pub async fn spawn_server(
    state: State<'_, AppState>
) -> Result<ServerInfo, String> {
    match spawn_and_wait().await {
        Ok(info) => {
            *state.server_port.lock().unwrap() = Some(info.port);
            if let Some(pid) = info.pid {
                *state.server_pid.lock().unwrap() = Some(pid);
            }
            Ok(info)
        }
        Err(e) => Err(e.to_string()),
    }
}

#[tauri::command]
pub async fn check_health(port: u16) -> Result<bool, String> {
    check_health(port)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn stop_server(
    state: State<'_, AppState>
) -> Result<(), String> {
    let pid = state.server_pid.lock().unwrap().take();
    if let Some(pid) = pid {
        // Implementation depends on opencode_client_core
        // For now, just clear state
        *state.server_port.lock().unwrap() = None;
    }
    Ok(())
}
```

## Step 4: Create Blazor WASM Frontend

### Initialize .NET Project

```bash
cd clients/tauri-blazor/frontend
dotnet new blazorwasm -n OpenCodeBlazor -o .
```

### `clients/tauri-blazor/frontend/OpenCodeBlazor.csproj`

```xml
<Project Sdk="Microsoft.NET.Sdk.BlazorWebAssembly">

  <PropertyGroup>
    <TargetFramework>net9.0</TargetFramework>
    <Nullable>enable</Nullable>
    <ImplicitUsings>enable</ImplicitUsings>
  </PropertyGroup>

  <ItemGroup>
    <PackageReference Include="Microsoft.AspNetCore.Components.WebAssembly" Version="9.0.0" />
    <PackageReference Include="Microsoft.AspNetCore.Components.WebAssembly.DevServer" Version="9.0.0" PrivateAssets="all" />
    <PackageReference Include="Radzen.Blazor" Version="5.0.0" />
    <PackageReference Include="Markdig" Version="0.37.0" />
  </ItemGroup>

  <PropertyGroup>
    <PublishDir>../src-tauri/frontend/</PublishDir>
  </PropertyGroup>

</Project>
```

### `clients/tauri-blazor/frontend/Program.cs`

```csharp
using Microsoft.AspNetCore.Components.Web;
using Microsoft.AspNetCore.Components.WebAssembly.Hosting;
using OpenCodeBlazor;
using OpenCodeBlazor.Services;
using Radzen;

var builder = WebAssemblyHostBuilder.CreateDefault(args);
builder.RootComponents.Add<App>("#app");
builder.RootComponents.Add<HeadOutlet>("head::after");

// Services
builder.Services.AddScoped<IServerService, ServerService>();
builder.Services.AddScoped<ISessionService, SessionService>();

// Radzen services
builder.Services.AddRadzenComponents();

await builder.Build().RunAsync();
```

### `clients/tauri-blazor/frontend/_Imports.razor`

```razor
@using System.Net.Http
@using System.Net.Http.Json
@using Microsoft.AspNetCore.Components.Forms
@using Microsoft.AspNetCore.Components.Routing
@using Microsoft.AspNetCore.Components.Web
@using Microsoft.AspNetCore.Components.WebAssembly.Http
@using Microsoft.JSInterop
@using OpenCodeBlazor
@using OpenCodeBlazor.Components
@using OpenCodeBlazor.Pages
@using OpenCodeBlazor.Services
@using Radzen
@using Radzen.Blazor
```

### `clients/tauri-blazor/frontend/Services/IServerService.cs`

```csharp
namespace OpenCodeBlazor.Services;

public record ServerInfo(int Port, int? Pid, string Url);

public interface IServerService
{
    Task<ServerInfo?> DiscoverServerAsync();
    Task<ServerInfo> SpawnServerAsync();
    Task<bool> CheckHealthAsync(int port);
    Task StopServerAsync();
}
```

### `clients/tauri-blazor/frontend/Services/ServerService.cs`

```csharp
using Microsoft.JSInterop;

namespace OpenCodeBlazor.Services;

public class ServerService : IServerService
{
    private readonly IJSRuntime _jsRuntime;

    public ServerService(IJSRuntime jsRuntime)
    {
        _jsRuntime = jsRuntime;
    }

    public async Task<ServerInfo?> DiscoverServerAsync()
    {
        try
        {
            // Directly invoke Tauri command via Blazor's IJSRuntime
            // NO custom JavaScript - this calls into Tauri's bundled JS
            var result = await _jsRuntime.InvokeAsync<ServerInfo?>(
                "window.__TAURI__.core.invoke",
                "discover_server"
            );
            return result;
        }
        catch (Exception ex)
        {
            Console.WriteLine($"Error discovering server: {ex.Message}");
            return null;
        }
    }

    public async Task<ServerInfo> SpawnServerAsync()
    {
        // Pure C# → Tauri IPC, no custom JS layer
        return await _jsRuntime.InvokeAsync<ServerInfo>(
            "window.__TAURI__.core.invoke",
            "spawn_server"
        );
    }

    public async Task<bool> CheckHealthAsync(int port)
    {
        // Passing parameters via anonymous object - serialized by Blazor
        return await _jsRuntime.InvokeAsync<bool>(
            "window.__TAURI__.core.invoke",
            "check_health",
            new { port }
        );
    }

    public async Task StopServerAsync()
    {
        await _jsRuntime.InvokeVoidAsync(
            "window.__TAURI__.core.invoke",
            "stop_server"
        );
    }
}
```

**Key Points:**

- ✅ Uses Blazor's `IJSRuntime` to call Tauri's bundled JavaScript
- ✅ No custom JS wrapper - direct `window.__TAURI__.core.invoke()` calls
- ✅ Blazor handles JSON serialization/deserialization automatically
- ✅ Type-safe with C# `Task<T>` return types
- ❌ No `wwwroot/js/tauri-helper.js` or similar - NOT ALLOWED

### `clients/tauri-blazor/frontend/Pages/Home.razor`

```razor
@page "/"
@inject IServerService ServerService

<PageTitle>OpenCode</PageTitle>

<RadzenStack Gap="1rem" Style="padding: 2rem;">
    <RadzenCard>
        <RadzenText TextStyle="TextStyle.H3">OpenCode Desktop Client</RadzenText>
        <RadzenText TextStyle="TextStyle.Body1">Tauri + Blazor Edition</RadzenText>
    </RadzenCard>

    <RadzenCard>
        <RadzenText TextStyle="TextStyle.H5">Server Status</RadzenText>

        @if (_serverInfo != null)
        {
            <RadzenText>Server running on port: @_serverInfo.Port</RadzenText>
            <RadzenText>URL: @_serverInfo.Url</RadzenText>
        }
        else if (_isDiscovering)
        {
            <RadzenText>Discovering server...</RadzenText>
        }
        else
        {
            <RadzenText>No server found</RadzenText>
        }

        <RadzenStack Orientation="Orientation.Horizontal" Gap="0.5rem" Style="margin-top: 1rem;">
            <RadzenButton Text="Discover Server" Click="DiscoverServer" Disabled="_isDiscovering" />
            <RadzenButton Text="Spawn Server" Click="SpawnServer" Disabled="_isSpawning" />
        </RadzenStack>
    </RadzenCard>
</RadzenStack>

@code {
    private ServerInfo? _serverInfo;
    private bool _isDiscovering;
    private bool _isSpawning;

    protected override async Task OnInitializedAsync()
    {
        await DiscoverServer();
    }

    private async Task DiscoverServer()
    {
        _isDiscovering = true;
        _serverInfo = await ServerService.DiscoverServerAsync();
        _isDiscovering = false;
    }

    private async Task SpawnServer()
    {
        _isSpawning = true;
        try
        {
            _serverInfo = await ServerService.SpawnServerAsync();
        }
        catch (Exception ex)
        {
            Console.WriteLine($"Error spawning server: {ex.Message}");
        }
        finally
        {
            _isSpawning = false;
        }
    }
}
```

## Step 5: Build Orchestration with Justfile

### `clients/tauri-blazor/justfile`

```makefile
# Build Blazor frontend and publish to Tauri directory
build-frontend:
    cd frontend && dotnet publish -c Release

# Run in development mode
dev: build-frontend
    cd src-tauri && cargo tauri dev

# Build production bundle
build: build-frontend
    cd src-tauri && cargo tauri build

# Clean build artifacts
clean:
    cd frontend && dotnet clean
    cd src-tauri && cargo clean
    rm -rf src-tauri/frontend

# Install dependencies
install:
    cd frontend && dotnet restore
    cd src-tauri && cargo fetch
```

## Step 6: Build and Run

```bash
cd clients/tauri-blazor

# Install dependencies
just install

# Build and run in development mode
just dev
```

## Next Steps

Once the basic scaffold is working:

1. **Add session management** - Implement chat session creation and management
2. **Build chat UI** - Create message list, input box, streaming support
3. **Integrate markdown** - Use Markdig for rendering Claude responses
4. **Add authentication** - OAuth flow for Anthropic, API key management
5. **Implement settings** - Model selection, provider configuration
6. **Add audio/STT** - Decide between web APIs or Tauri command wrapper

## What About Cognexus's Custom JavaScript?

**You may notice Cognexus has `wwwroot/js/renderer-helper.js` - why?**

Cognexus has a unique requirement: it uses **WGPU** for GPU-accelerated rendering in a `<canvas>` element. The custom JavaScript is needed to:

- Initialize the WebGPU/WebGL context
- Bridge between Blazor and the Rust WASM renderer
- Handle low-level canvas operations

**OpenCode does NOT need this because:**

- ❌ No custom GPU rendering engine
- ❌ No low-level canvas manipulation
- ✅ Pure UI application using standard HTML/CSS
- ✅ Radzen components handle all rendering
- ✅ Markdown rendering via C# libraries (Markdig)

**Conclusion:**

- Cognexus's custom JS is for **advanced graphics rendering** - not applicable to OpenCode
- OpenCode is a **standard CRUD/chat application** - no custom JS needed
- All Tauri IPC can be done via C# `IJSRuntime` calling Tauri's own bundled JS
- **DO NOT create custom JavaScript files for OpenCode**

## Troubleshooting

### Blazor Not Loading in Tauri

**Problem:** Tauri shows blank screen

**Solution:** Ensure Blazor was published correctly:

```bash
ls src-tauri/frontend/
# Should contain: index.html, _framework/, css/, js/
```

### Tauri Commands Not Working

**Problem:** `window.__TAURI__ is undefined`

**Solution:** Check `tauri.conf.json` has `"withGlobalTauri": true`

### CORS Issues with Server

**Problem:** Blazor can't reach OpenCode server

**Solution:** For development, the server should allow localhost origins. Check server CORS configuration.

## Reference Implementation

See the Cognexus project for a working example **WITH MODIFICATIONS**:

- Location: `/Users/tony/git/cognexus`
- Tech stack: Tauri 2.9.5 + Blazor WASM .NET 10.0
- Build system: justfile
- Architecture: Similar workspace structure with shared Rust crates

**CRITICAL DIFFERENCE:**

- ❌ Cognexus has `wwwroot/js/renderer-helper.js` - **DO NOT REPLICATE THIS**
- ❌ Cognexus uses custom JavaScript for WGPU rendering - **NOT APPLICABLE**
- ✅ OpenCode will use ZERO custom JavaScript
- ✅ All Tauri IPC via C# `IJSRuntime` only
- ✅ All UI via Blazor components (Radzen), no manual DOM manipulation

## Resources

- [Tauri Documentation](https://v2.tauri.app/)
- [Blazor WebAssembly Guide](https://learn.microsoft.com/en-us/aspnet/core/blazor/)
- [Radzen Blazor Components](https://blazor.radzen.com/)
- [ADR-0001: Tauri + Blazor Desktop Client](../docs/adr/0001-tauri-blazor-desktop-client.md)
