# Next Session: Blazor Frontend Scaffold & Server Integration

## Quick Context

**What We Completed (Session 2 - 2026-01-03):**

- ✅ Created Tauri 2.9.5 project at `apps/desktop/opencode/`
- ✅ Implemented actor-based state management (race-free)
- ✅ Created 4 Tauri commands: `discover_server`, `spawn_server`, `check_health`, `stop_server`
- ✅ Production-grade logging infrastructure
- ✅ Renamed `common/` → `models/` for teaching clarity
- ✅ Tested all commands via HTML test frontend
- ✅ Clippy clean with `-D warnings`

**Current State:**

- Tauri backend fully functional ✅
- Can discover/spawn/stop OpenCode server via Tauri commands ✅
- State management working (actor pattern) ✅
- **BUT:** No Blazor frontend yet - need to create .NET project and C# service layer

---

## Your Mission: Session 3 - Blazor Frontend & Service Layer

Build the Blazor WebAssembly frontend that calls Tauri commands via C# IJSRuntime, following the zero-custom-JavaScript policy.

### Step 1: Initialize Blazor WASM Project

**Goal:** Create a .NET 9 Blazor WASM project with proper dependencies

**Tasks:**

1. Create `frontend/` directory in `clients/tauri-blazor/apps/desktop/opencode/frontend/`

2. Initialize .NET 9 Blazor WASM project:

   ```bash
   cd clients/tauri-blazor/apps/desktop/opencode
   dotnet new blazorwasm -n OpenCodeBlazor -o frontend
   ```

3. Update `frontend/OpenCodeBlazor.csproj` with dependencies:

   ```xml
   <Project Sdk="Microsoft.NET.Sdk.BlazorWebAssembly">
     <PropertyGroup>
       <TargetFramework>net9.0</TargetFramework>
       <Nullable>enable</Nullable>
       <ImplicitUsings>enable</ImplicitUsings>
     </PropertyGroup>

     <ItemGroup>
       <!-- Radzen Blazor Components -->
       <PackageReference Include="Radzen.Blazor" Version="5.8.8" />

       <!-- Markdown Rendering -->
       <PackageReference Include="Markdig" Version="0.38.0" />

       <!-- Blazor WASM -->
       <PackageReference Include="Microsoft.AspNetCore.Components.WebAssembly" Version="9.0.0" />
       <PackageReference Include="Microsoft.AspNetCore.Components.WebAssembly.DevServer" Version="9.0.0" PrivateAssets="all" />
     </ItemGroup>
   </Project>
   ```

4. Configure publish to output to `wwwroot/`:

   ```xml
   <PropertyGroup>
     <PublishDir>wwwroot</PublishDir>
   </PropertyGroup>
   ```

5. Update `tauri.conf.json` to point to Blazor output:

   ```json
   {
     "build": {
       "frontendDist": "./frontend/wwwroot"
     }
   }
   ```

6. Verify project builds: `dotnet build`

**Technical Details:**

- Use .NET 9 (latest stable, better performance than .NET 8)
- Radzen 5.8.8+ for UI components (no custom DOM manipulation needed)
- Markdig for markdown rendering (will be used in Session 4 for chat)
- Publish to `wwwroot/` so Tauri can serve static files

---

### Step 2: Configure Blazor for Tauri Integration

**Goal:** Set up dependency injection and Tauri JSInterop

**Tasks:**

1. Update `frontend/Program.cs` to configure services:

   ```csharp
   using Microsoft.AspNetCore.Components.Web;
   using Microsoft.AspNetCore.Components.WebAssembly.Hosting;
   using OpenCodeBlazor;
   using Radzen;

   var builder = WebAssemblyHostBuilder.CreateDefault(args);
   builder.RootComponents.Add<App>("#app");
   builder.RootComponents.Add<HeadOutlet>("head::after");

   // Radzen services
   builder.Services.AddRadzenComponents();

   // OpenCode services (we'll create these in Step 2)
   builder.Services.AddScoped<IServerService, ServerService>();

   await builder.Build().RunAsync();
   ```

2. Update `frontend/wwwroot/index.html` to include Tauri API:

   ```html
   <!DOCTYPE html>
   <html lang="en">
     <head>
       <meta charset="utf-8" />
       <meta name="viewport" content="width=device-width, initial-scale=1.0" />
       <title>OpenCode</title>
       <base href="/" />

       <!-- Radzen CSS -->
       <link rel="stylesheet" href="_content/Radzen.Blazor/css/material-base.css" />

       <link href="css/app.css" rel="stylesheet" />
       <link href="OpenCodeBlazor.styles.css" rel="stylesheet" />
     </head>
     <body>
       <div id="app">
         <div class="loading">Loading...</div>
       </div>

       <!-- Blazor framework -->
       <script src="_framework/blazor.webassembly.js"></script>
     </body>
   </html>
   ```

   **Important:** NO custom JavaScript! Tauri API is available via C# IJSRuntime.

3. Create `frontend/Imports.razor` for global using directives:

   ```razor
   @using System.Net.Http
   @using System.Net.Http.Json
   @using Microsoft.AspNetCore.Components.Forms
   @using Microsoft.AspNetCore.Components.Routing
   @using Microsoft.AspNetCore.Components.Web
   @using Microsoft.AspNetCore.Components.Web.Virtualization
   @using Microsoft.AspNetCore.Components.WebAssembly.Http
   @using Microsoft.JSInterop
   @using OpenCodeBlazor
   @using OpenCodeBlazor.Services
   @using Radzen
   @using Radzen.Blazor
   ```

**Technical Details:**

- Radzen provides DialogService, NotificationService, etc. via DI
- `IJSRuntime` is Blazor's built-in JavaScript interop mechanism
- NO custom JavaScript files (all Tauri IPC via C# wrappers)

---

### Step 3: Create Server Service Layer

**Goal:** Create C# service that wraps Tauri commands using IJSRuntime

**Tasks:**

1. Create `frontend/Models/ServerInfo.cs` (matches Rust struct):

   ```csharp
   namespace OpenCodeBlazor.Models;

   public record ServerInfo(
       uint Pid,
       string Host,
       ushort Port
   );
   ```

2. Create `frontend/Services/IServerService.cs`:

   ```csharp
   namespace OpenCodeBlazor.Services;

   using OpenCodeBlazor.Models;

   public interface IServerService
   {
       /// <summary>
       /// Discovers a running OpenCode server on localhost.
       /// </summary>
       Task<ServerInfo?> DiscoverServerAsync();

       /// <summary>
       /// Spawns a new OpenCode server and waits for health check.
       /// </summary>
       Task<ServerInfo> SpawnServerAsync();

       /// <summary>
       /// Checks if the currently connected server is healthy.
       /// </summary>
       Task<bool> CheckHealthAsync();

       /// <summary>
       /// Stops the currently connected server.
       /// </summary>
       Task StopServerAsync();
   }
   ```

3. Create `frontend/Services/ServerService.cs`:

   ```csharp
   namespace OpenCodeBlazor.Services;

   using Microsoft.JSInterop;
   using OpenCodeBlazor.Models;
   using System.Text.Json;

   public class ServerService : IServerService
   {
       private readonly IJSRuntime _jsRuntime;

       public ServerService(IJSRuntime jsRuntime)
       {
           _jsRuntime = jsRuntime;
       }

       public async Task<ServerInfo?> DiscoverServerAsync()
       {
           // Call Tauri command via IJSRuntime
           // Pattern: await _jsRuntime.InvokeAsync<T>("__TAURI__.invoke", "command_name")
           var result = await _jsRuntime.InvokeAsync<JsonElement>(
               "eval",
               "window.__TAURI__.invoke('discover_server')"
           );

           // Parse result (may be null if no server found)
           if (result.ValueKind == JsonValueKind.Null)
               return null;

           return JsonSerializer.Deserialize<ServerInfo>(result.GetRawText());
       }

       public async Task<ServerInfo> SpawnServerAsync()
       {
           // Similar pattern for spawn_server
           var result = await _jsRuntime.InvokeAsync<JsonElement>(
               "eval",
               "window.__TAURI__.invoke('spawn_server')"
           );

           return JsonSerializer.Deserialize<ServerInfo>(result.GetRawText())
               ?? throw new InvalidOperationException("Failed to spawn server");
       }

       public async Task<bool> CheckHealthAsync()
       {
           // Similar pattern for check_health
           return await _jsRuntime.InvokeAsync<bool>(
               "eval",
               "window.__TAURI__.invoke('check_health')"
           );
       }

       public async Task StopServerAsync()
       {
           // Similar pattern for stop_server
           await _jsRuntime.InvokeAsync<object>(
               "eval",
               "window.__TAURI__.invoke('stop_server')"
           );
       }
   }
   ```

   **Note:** This uses `eval` as a workaround. We may need a better approach (see Technical Details).

**Technical Details:**

- **Challenge:** Tauri API is at `window.__TAURI__.invoke()`, but IJSRuntime needs a function name
- **Options:**
  1. Use `eval` (works but not ideal)
  2. Create a tiny JS wrapper file (violates zero-custom-JS policy slightly)
  3. Use Tauri events instead of commands (more complex)
- **Recommended:** Start with `eval`, verify it works, then refactor if needed
- All Tauri commands return `Promise<T>`, which maps to C# `Task<T>`
- Error handling: Tauri errors reject the Promise, which throws in C#

---

### Step 4: Build Basic Server Status UI

**Goal:** Create a simple UI to test server discovery/spawn from Blazor

**Tasks:**

1. Create `frontend/Pages/Home.razor`:

   ```razor
   @page "/"
   @inject IServerService ServerService
   @inject NotificationService NotificationService

   <PageTitle>OpenCode - Home</PageTitle>

   <RadzenCard>
       <RadzenStack Gap="1rem">
           <RadzenText TextStyle="TextStyle.H3">Server Status</RadzenText>

           @if (serverInfo != null)
           {
               <RadzenAlert AlertStyle="AlertStyle.Success">
                   <RadzenText>
                       Server running: PID @serverInfo.Pid on @serverInfo.Host:@serverInfo.Port
                   </RadzenText>
               </RadzenAlert>
           }
           else
           {
               <RadzenAlert AlertStyle="AlertStyle.Warning">
                   No server connected
               </RadzenAlert>
           }

           <RadzenStack Orientation="Orientation.Horizontal" Gap="0.5rem">
               <RadzenButton Text="Discover Server"
                             Click="OnDiscoverAsync"
                             IsBusy="isLoading" />

               <RadzenButton Text="Spawn Server"
                             Click="OnSpawnAsync"
                             IsBusy="isLoading"
                             Variant="Variant.Filled" />

               <RadzenButton Text="Check Health"
                             Click="OnCheckHealthAsync"
                             IsBusy="isLoading"
                             Disabled="@(serverInfo == null)" />

               <RadzenButton Text="Stop Server"
                             Click="OnStopAsync"
                             IsBusy="isLoading"
                             ButtonStyle="ButtonStyle.Danger"
                             Disabled="@(serverInfo == null)" />
           </RadzenStack>
       </RadzenStack>
   </RadzenCard>

   @code {
       private ServerInfo? serverInfo;
       private bool isLoading;

       protected override async Task OnInitializedAsync()
       {
           // Try to discover server on startup
           await OnDiscoverAsync();
       }

       private async Task OnDiscoverAsync()
       {
           isLoading = true;
           try
           {
               serverInfo = await ServerService.DiscoverServerAsync();

               if (serverInfo != null)
                   NotificationService.Notify(NotificationSeverity.Success, "Server discovered");
               else
                   NotificationService.Notify(NotificationSeverity.Info, "No server found");
           }
           catch (Exception ex)
           {
               NotificationService.Notify(NotificationSeverity.Error, "Discovery failed", ex.Message);
           }
           finally
           {
               isLoading = false;
           }
       }

       private async Task OnSpawnAsync()
       {
           isLoading = true;
           try
           {
               serverInfo = await ServerService.SpawnServerAsync();
               NotificationService.Notify(NotificationSeverity.Success, "Server spawned");
           }
           catch (Exception ex)
           {
               NotificationService.Notify(NotificationSeverity.Error, "Spawn failed", ex.Message);
           }
           finally
           {
               isLoading = false;
           }
       }

       private async Task OnCheckHealthAsync()
       {
           isLoading = true;
           try
           {
               var healthy = await ServerService.CheckHealthAsync();
               var severity = healthy ? NotificationSeverity.Success : NotificationSeverity.Warning;
               NotificationService.Notify(severity, healthy ? "Server healthy" : "Server unhealthy");
           }
           catch (Exception ex)
           {
               NotificationService.Notify(NotificationSeverity.Error, "Health check failed", ex.Message);
           }
           finally
           {
               isLoading = false;
           }
       }

       private async Task OnStopAsync()
       {
           isLoading = true;
           try
           {
               await ServerService.StopServerAsync();
               serverInfo = null;
               NotificationService.Notify(NotificationSeverity.Success, "Server stopped");
           }
           catch (Exception ex)
           {
               NotificationService.Notify(NotificationSeverity.Error, "Stop failed", ex.Message);
           }
           finally
           {
               isLoading = false;
           }
       }
   }
   ```

2. Update `frontend/App.razor` to include Radzen components:

   ```razor
   <RadzenDialog />
   <RadzenNotification />
   <RadzenContextMenu />
   <RadzenTooltip />

   <Router AppAssembly="@typeof(App).Assembly">
       <Found Context="routeData">
           <RouteView RouteData="@routeData" DefaultLayout="@typeof(MainLayout)" />
           <FocusOnNavigate RouteData="@routeData" Selector="h1" />
       </Found>
       <NotFound>
           <PageTitle>Not found</PageTitle>
           <LayoutView Layout="@typeof(MainLayout)">
               <p role="alert">Sorry, there's nothing at this address.</p>
           </LayoutView>
       </NotFound>
   </Router>
   ```

3. Test the full flow:

   ```bash
   # Build and publish Blazor
   cd clients/tauri-blazor/apps/desktop/opencode/frontend
   dotnet publish -c Release

   # Run Tauri app
   cd ..
   cargo tauri dev
   ```

4. Verify functionality:
   - App launches with Blazor UI ✅
   - "Discover Server" button works
   - "Spawn Server" button spawns server and updates UI
   - "Check Health" button returns correct status
   - "Stop Server" button stops server and clears UI
   - Radzen notifications show success/error messages

**Technical Details:**

- Radzen components provide Material Design styling out of the box
- `NotificationService` shows toast notifications (no custom JS needed)
- `IsBusy` prop on buttons shows loading spinner automatically
- All state management in component (`serverInfo`, `isLoading`)

---

## Success Criteria for Session 3

- [ ] Blazor WASM project created in `frontend/` directory
- [ ] `dotnet build` succeeds
- [ ] `dotnet publish` outputs to `wwwroot/`
- [ ] Tauri app loads Blazor UI successfully
- [ ] `IServerService` and `ServerService` implemented
- [ ] Home.razor displays server status
- [ ] "Discover Server" button calls Tauri command via C#
- [ ] "Spawn Server" button spawns server and updates UI
- [ ] "Check Health" button works correctly
- [ ] "Stop Server" button stops server and clears state
- [ ] Radzen notifications show success/error messages
- [ ] NO custom JavaScript files created (all IPC via C# IJSRuntime)

---

## Key Files to Reference

**Existing (Read these first):**

- `apps/desktop/opencode/src/commands/server.rs` - Tauri commands to call
- `apps/desktop/opencode/src/state.rs` - State management (for understanding)
- `models/src/server_info.rs` - Rust ServerInfo struct (match in C#)
- `apps/desktop/opencode/tauri.conf.json` - Tauri config (update frontendDist)

**To Create:**

- `frontend/OpenCodeBlazor.csproj` - .NET project file
- `frontend/Program.cs` - DI configuration
- `frontend/wwwroot/index.html` - Entry HTML (Blazor + Radzen CSS)
- `frontend/Models/ServerInfo.cs` - C# model matching Rust
- `frontend/Services/IServerService.cs` - Service interface
- `frontend/Services/ServerService.cs` - Service implementation (IJSRuntime)
- `frontend/Pages/Home.razor` - Server status UI
- `frontend/App.razor` - Root component with Radzen services
- `frontend/Imports.razor` - Global using directives

**Reference (for patterns):**

- Cognexus Blazor example (if available)
- Radzen documentation for component usage
- Blazor IJSRuntime documentation for Tauri interop

---

## Important Reminders

1. **Zero custom JavaScript** - All Tauri IPC via C# IJSRuntime only
2. **.NET 9.0** - Use latest stable .NET version
3. **Radzen components** - Use for all UI (buttons, cards, notifications)
4. **Error handling** - Show user-friendly error messages via Radzen notifications
5. **Loading states** - Use `IsBusy` prop on buttons for better UX
6. **Publish to wwwroot/** - Tauri expects static files in `frontendDist` path
7. **Test as you go** - Verify each command works before moving on
8. **Documentation** - XML doc comments on all public APIs

---

## Technical Constraints

- **.NET Version:** 9.0+
- **Blazor Mode:** WebAssembly (NOT Server)
- **Zero Custom JavaScript:** All Tauri IPC via C# IJSRuntime
- **UI Framework:** Radzen Blazor Components only
- **Markdown:** Markdig (for Session 4, add dependency now)
- **CSP:** null (required for Blazor WASM, already configured in tauri.conf.json)

---

## Estimated Token Budget

**~120K tokens:**

- Reading context: ~15K tokens (Tauri backend + docs)
- Blazor project setup: ~15K tokens
- Dependency configuration: ~10K tokens
- Service layer implementation: ~25K tokens
- UI components: ~30K tokens
- Testing/verification: ~15K tokens
- Documentation: ~10K tokens

---

## Known Challenges & Solutions

**Challenge 1:** IJSRuntime can't directly call `window.__TAURI__.invoke()`

**Solution:** Use `eval` wrapper initially, refactor later if needed:

```csharp
await _jsRuntime.InvokeAsync<T>("eval", "window.__TAURI__.invoke('command_name')")
```

**Challenge 2:** JSON deserialization from Tauri commands

**Solution:** Use `JsonElement` intermediate type, then deserialize:

```csharp
var json = await _jsRuntime.InvokeAsync<JsonElement>(...);
return JsonSerializer.Deserialize<ServerInfo>(json.GetRawText());
```

**Challenge 3:** Blazor publish output not updating in Tauri

**Solution:** Configure `PublishDir` in csproj, or use build script to copy files

---

**Start with:** "Let me read the Tauri commands to understand the API, then initialize the Blazor WASM project with proper dependencies."
