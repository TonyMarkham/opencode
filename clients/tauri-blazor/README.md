# OpenCode Tauri-Blazor Desktop Client

This is a cross-platform desktop client for OpenCode built with Tauri (Rust backend) and Blazor (.NET frontend).

## Project Structure

This feature follows the Cognexus project layout pattern:

```
clients/tauri-blazor/                        # Feature root (self-contained)
├── apps/
│   └── desktop/
│       └── opencode/                        # Tauri desktop application
│           ├── src/                         # Rust Tauri code (main.rs, commands, state)
│           ├── frontend/                    # Built Blazor output (wwwroot)
│           ├── icons/                       # Application icons
│           ├── Cargo.toml                   # Tauri app manifest
│           ├── build.rs                     # Tauri build script
│           └── tauri.conf.json              # Tauri configuration
├── backend/                                 # Shared Rust crates for this feature
│   └── client-core/                         # Server discovery and API client logic
│       ├── src/
│       │   ├── lib.rs                       # Public API exports
│       │   ├── discovery/
│       │   │   ├── mod.rs                   # Port override logic
│       │   │   ├── process.rs               # ServerInfo, discover(), stop_pid(), check_health()
│       │   │   └── spawn.rs                 # spawn_and_wait()
│       │   └── error/
│       │       ├── mod.rs
│       │       ├── discovery.rs             # DiscoveryError enum
│       │       └── spawn.rs                 # SpawnError enum
│       └── Cargo.toml                       # Shared crate manifest
├── frontend/                                # Blazor source code
│   └── opencode/                            # Blazor project
│       ├── Pages/                           # Blazor pages/components
│       ├── Layout/                          # Layout components
│       ├── Services/                        # C# services for Tauri interop
│       ├── wwwroot/                         # Static assets
│       └── OpenCode.csproj                  # Blazor project file
├── Cargo.toml                               # Workspace root for this feature
└── README.md                                # This file
```

## Architecture

- **Tauri Backend** (`apps/desktop/opencode/src/`): Rust code handling system integration, server discovery, and command execution
- **Shared Core** (`backend/client-core/`): Reusable Rust logic shared between egui and Tauri clients
- **Blazor Frontend** (`frontend/opencode/`): C# Blazor WebAssembly UI compiled to the `apps/desktop/opencode/frontend/` directory

## Development

(Setup and build instructions will be added as implementation progresses)

## References

- Architecture Decision Record: `/docs/adr/0001-tauri-blazor-desktop-client.md`
- Session Plan: `SESSION_PLAN.md`
- Cognexus reference project: `/Users/tony/git/cognexus`
