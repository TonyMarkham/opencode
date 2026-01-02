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
│       │   │   ├── mod.rs                   # Discovery module
│       │   │   ├── process.rs               # ServerInfo, discover(), stop_pid(), check_health()
│       │   │   └── spawn.rs                 # spawn_and_wait()
│       │   ├── error/
│       │   │   ├── mod.rs                   # Error module
│       │   │   ├── discovery.rs             # DiscoveryError enum
│       │   │   └── spawn.rs                 # SpawnError enum
│       │   └── tests/                       # Unit tests (mirror src/ structure)
│       ├── integration_tests/               # Integration tests
│       │   ├── mod.rs
│       │   ├── discovery/
│       │   └── error/
│       └── Cargo.toml                       # client-core crate manifest
├── common/                                  # Shared utilities across crates
│   ├── src/
│   │   ├── lib.rs                           # Public API exports
│   │   ├── error/
│   │   │   ├── mod.rs                       # Error utilities module
│   │   │   └── error_location.rs            # ErrorLocation trait
│   │   └── tests/                           # Unit tests
│   │       └── error_location.rs
│   └── Cargo.toml                           # common crate manifest
├── frontend/                                # Blazor source code
│   └── opencode/                            # Blazor project
│       ├── Pages/                           # Blazor pages/components
│       ├── Layout/                          # Layout components
│       ├── Services/                        # C# services for Tauri interop
│       ├── wwwroot/                         # Static assets
│       └── OpenCode.csproj                  # Blazor project file
├── Cargo.toml                               # Workspace root manifest
└── README.md                                # This file
```

## Architecture

- **Tauri Backend** (`apps/desktop/opencode/src/`): Rust code handling system integration, server discovery, and command execution
- **Shared Core** (`backend/client-core/`): Reusable Rust logic for server discovery, spawning, and health checks
  - Unit tests in `src/tests/` (mirror source structure)
  - Integration tests in `integration_tests/` (configured via `[[test]]` in Cargo.toml)
- **Common Utilities** (`common/`): Shared utilities like ErrorLocation trait used across all crates
  - Unit tests in `src/tests/`
- **Blazor Frontend** (`frontend/opencode/`): C# Blazor WebAssembly UI compiled to the `apps/desktop/opencode/frontend/` directory

## Development

(Setup and build instructions will be added as implementation progresses)

## References

- Architecture Decision Record: `/docs/adr/0001-tauri-blazor-desktop-client.md`
- Session Plan: `SESSION_PLAN.md`
- Cognexus reference project: `/Users/tony/git/cognexus`
