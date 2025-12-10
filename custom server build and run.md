# Custom OpenCode LLM Server - Build and Run Guide

This guide explains how to build and run the OpenCode LLM server independently from the GUI client.

## Prerequisites

### Required Dependencies
- **Bun** - JavaScript runtime and package manager
- **Git** - For cloning and managing the repository
- **Node.js** (optional, but recommended for some tooling)

### Installation Commands
```bash
# Install Bun (if not already installed)
curl -fsSL https://bun.sh/install | bash

# Or using Homebrew on macOS
brew install bun

# IMPORTANT: This project requires Bun 1.3.3 exactly
# If you have a different version, you may need to downgrade:
bun install --bun 1.3.3

# Or use bunx to ensure the correct version:
bunx --bun 1.3.3 <command>
```

## Building the Custom Server

### Step 1: Clone the Repository
```bash
git clone https://github.com/sst/opencode.git
cd opencode
```

### Step 2: Install Dependencies
```bash
bun install
```

### Step 3: Build the Server
Navigate to the opencode package directory and run the build:

```bash
cd packages/opencode
OPENCODE_VERSION=1.0.134.pre-1 bun run ./script/build.ts
```

You can omit `OPENCODE_VERSION=...` if you just want the default latest version.

### Build Options
- **Full build** (default): Builds for all platforms (Linux, macOS, Windows)
- **Single platform build**: Build only for your current platform
  ```bash
  bun run build --single
  ```
- **Skip install**: Skip dependency installation during build
  ```bash
  bun run build --skip-install
  ```

### Building with a specific version
By default, the build script derives the version automatically. To force a specific or pre-release version, set `OPENCODE_VERSION` when building.

The explicit form (what actually runs) is:

```bash
cd packages/opencode
OPENCODE_VERSION=1.0.134.pre-1 bun run ./script/build.ts
```

You can also use the shorthand, which calls the same script via the `build` npm script:

```bash
cd packages/opencode
OPENCODE_VERSION=1.0.134.pre-1 bun run build
```

Replace `1.0.134.pre-1` with your desired version string.

### Build Output
After building, you'll find the compiled binaries in:
```
packages/opencode/dist/
├── opencode-darwin-arm64/     # macOS Apple Silicon
├── opencode-darwin-x64/       # macOS Intel
├── opencode-linux-arm64/      # Linux ARM64
├── opencode-linux-x64/        # Linux x64
└── opencode-windows-x64/      # Windows x64
```

Each platform directory contains:
- `bin/opencode` - The executable server binary
- `package.json` - Package metadata

### Step 4: Install or update your default `opencode` binary
If you want your shell `opencode` command to use this custom build by default, copy the binary for your platform into your home `~/.opencode` directory. For example, on macOS Apple Silicon:

```bash
mkdir -p ~/.opencode/bin
cp packages/opencode/dist/opencode-darwin-arm64/bin/opencode ~/.opencode/bin/opencode
chmod +x ~/.opencode/bin/opencode
```

Then verify:

```bash
opencode --version
```

## Running the Server Independently

### Step 1: Locate Your Binary
Find the appropriate binary for your system in the `dist/` directory. For example, on macOS Apple Silicon:
```bash
cd packages/opencode/dist/opencode-darwin-arm64/bin
```

### Step 2: Make the Binary Executable (Unix systems)
```bash
chmod +x opencode
```

### Step 3: Run the Server
```bash
./opencode
```

### Server Configuration Options
The server supports various command-line options:

```bash
# Show help and all available options
./opencode --help

# Run with specific configuration
./opencode --port 3000 --host 0.0.0.0

# Run in development mode with verbose logging
./opencode --dev --verbose

# Specify custom model provider
./opencode --provider anthropic --model claude-3-5-sonnet-20241022
```

### Environment Variables
You can configure the server using environment variables for provider API keys and various feature flags.

```bash
# Set API keys (required for cloud models)
export ANTHROPIC_API_KEY="your-anthropic-key"
export OPENAI_API_KEY="your-openai-key"

# Example: optional feature/behavior flags
export OPENCODE_CONFIG="/path/to/opencode.config.json"   # custom config file
export OPENCODE_DISABLE_AUTOUPDATE=1                     # disable auto-updates

# Run the server
./opencode
```

Port and host are normally set via CLI flags:

```bash
./opencode --port 3000 --host 0.0.0.0
```

### Running as a Background Service
To run the server in the background:

```bash
# Using nohup (Unix systems)
nohup ./opencode > server.log 2>&1 &

# Using screen or tmux
screen -S opencode
./opencode
# Press Ctrl+A+D to detach

# Using systemd (Linux)
sudo systemctl start opencode
```

## Connecting to the Server

Once running, the server typically listens on:
- **Default port**: 3000
- **Default host**: localhost (127.0.0.1)

You can connect to it using:
- HTTP API endpoints
- WebSocket connections
- MCP (Model Context Protocol) clients
- Custom integrations

### Example API Connection
```bash
# Health check
curl http://localhost:3000/health

# Start a session
curl -X POST http://localhost:3000/session \
  -H "Content-Type: application/json" \
  -d '{"model": "claude-3-5-sonnet-20241022"}'
```

## Troubleshooting

### Common Issues

1. **Bun Version Mismatch**
   ```bash
   # Check current version
   bun --version
   
   # If not 1.3.3, downgrade:
   bun install --bun 1.3.3
   
   # Or use bunx for specific commands:
   bunx --bun 1.3.3 bun run build
   ```

2. **Permission Denied**
   ```bash
   chmod +x opencode
   ```

3. **Missing Dependencies**
   ```bash
   bun run build --skip-install  # Rebuild with dependencies
   ```

4. **Port Already in Use**
   ```bash
   # Kill existing process
   lsof -ti:3000 | xargs kill -9
   # Or use different port
   ./opencode --port 3001
   ```

5. **API Key Issues**
   ```bash
   # Set environment variables
   export ANTHROPIC_API_KEY="your-key"
   export OPENAI_API_KEY="your-key"
   ```

### Debug Mode
Run with debug flags for troubleshooting:
```bash
./opencode --debug --verbose --log-level debug
```

## Development Workflow

For ongoing development:

```bash
# Watch mode for automatic rebuilding
cd packages/opencode
bun run dev

# Test changes
bun run test

# Type checking
bun run typecheck
```

## Architecture Notes

- The server is built as a standalone binary using Bun
- It includes all necessary dependencies bundled
- No external Node.js runtime required for the built binary
- Supports multiple LLM providers (Anthropic, OpenAI, etc.)
- Built-in MCP (Model Context Protocol) support
- Cross-platform compatibility (Linux, macOS, Windows)