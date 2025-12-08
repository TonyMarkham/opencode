# Critical Issue: Orphaned Processes on Server Termination

## Severity: HIGH
**Status**: Unresolved  
**Platform**: macOS, Linux (any Unix-like system)

## Problem Description

When the OpenCode server process is terminated (e.g., when EGUI client closes), child processes spawned by the server continue running as orphaned processes. This affects **three categories** of processes:

1. **Bash tool subprocesses** - Commands executed via the bash tool (e.g., `npm run dev`, `sleep 300`)
2. **PTY sessions** - Interactive terminal sessions spawned via the PTY API
3. **LSP servers** - Language server processes (TypeScript, Deno, Vue, ESLint)

All three lead to process leaks and potential resource exhaustion.

## Root Cause

### Current Behavior

1. **EGUI client cleanup** (`clients/egui/src/app.rs:1079-1091`):
   - On exit, sends `SIGTERM` to OpenCode server via `stop_pid()`
   - Server process terminates immediately

2. **Multiple process types spawn children**:
   
   **Bash tool** (`packages/opencode/src/tool/bash.ts:216-224`):
   ```typescript
   const proc = spawn(params.command, {
       shell,
       cwd: Instance.directory,
       env: { ...process.env },
       stdio: ["ignore", "pipe", "pipe"],
       detached: process.platform !== "win32",  // ← Creates separate process group
   })
   ```
   
   **PTY sessions** (`packages/opencode/src/pty/index.ts:121`):
   ```typescript
   const ptyProcess = spawn(command, args, {
       name: "xterm-256color",
       cwd,
       env,
   })
   ```
   
   **LSP servers** (`packages/opencode/src/lsp/server.ts`):
   ```typescript
   // TypeScript LSP (line 94)
   spawn(BunProc.which(), ["x", "typescript-language-server", "--stdio"], { cwd: root })
   
   // Deno LSP (line 76)
   spawn(deno, ["lsp"], { cwd: root })
   
   // Vue, ESLint, etc.
   ```

3. **Process group isolation**:
   - `detached: true` places subprocess in its own process group
   - When parent receives SIGTERM, signal doesn't propagate to detached children
   - Children are reparented to PID 1 (launchd on macOS) and continue running

### What Leaks

**Bash Tools**:
- Long-running commands: dev servers (`npm run dev`), watchers, build processes
- Database queries, network requests
- Any command executed via bash tool

**PTY Sessions**:
- Interactive shell processes (zsh, bash, fish)
- Any commands running inside terminal sessions
- Background jobs started in terminals

**LSP Servers**:
- TypeScript language server processes
- Deno LSP processes
- Vue, ESLint, and other language servers
- Can be multiple instances per workspace/language

**Common Resources**:
- File descriptors: open files, sockets, pipes
- System resources: CPU cycles, memory allocations
- Zombie processes: completed but not reaped child processes
- Port bindings: dev servers, LSP servers on specific ports

### Example Scenarios

**Scenario A: Bash Tool Leak**
```bash
# User executes via agent:
$ npm run dev  # Starts Vite dev server on port 5173

# User closes EGUI before tool completes
# → EGUI kills OpenCode server
# → npm and Vite continue running in background
# → Port 5173 remains occupied
# → Next launch fails with "port already in use"
```

**Scenario B: PTY Session Leak**
```bash
# User opens terminal in EGUI, starts long process
$ docker compose up

# User closes EGUI
# → Shell process and docker compose continue running
# → Docker containers keep running unintentionally
# → Resources consumed until manual kill
```

**Scenario C: LSP Server Leak**
```bash
# User works on TypeScript project
# → Server spawns typescript-language-server (PID 12345)
# → User closes EGUI
# → LSP server keeps running indefinitely
# → Multiple launches accumulate LSP processes
# → `ps aux | grep typescript-language-server` shows 5+ instances
```

## Technical Details

### Signal Propagation on Unix

- `SIGTERM` sent to process group leader does NOT propagate to detached children
- Only non-detached processes in the same process group receive the signal
- Detached processes require explicit signal delivery via negative PID: `process.kill(-pid, signal)`

### Current Cleanup Mechanisms

**Bash Tool** has a `killTree()` function (lines 255-283) that properly kills process trees:

```typescript
const killTree = async () => {
    // ... checks ...
    
    if (process.platform === "win32") {
        // Uses taskkill /t for tree kill
    } else {
        // Sends to negative PID (entire process group)
        process.kill(-pid, "SIGTERM")
        await Bun.sleep(SIGKILL_TIMEOUT_MS)
        if (!exited) {
            process.kill(-pid, "SIGKILL")
        }
    }
}
```

**PTY module** has cleanup in `Instance.state()` (lines 91-101):

```typescript
async (sessions) => {
    for (const session of sessions.values()) {
        try {
            session.process.kill()
        } catch {}
        for (const ws of session.subscribers) {
            ws.close()
        }
    }
    sessions.clear()
}
```

**LSP servers** have no explicit cleanup mechanism.

**Problem**: These cleanups only run when:
- Bash: Tool times out or is explicitly aborted via `ctx.abort`
- PTY: Instance state is explicitly cleared
- LSP: Never (no cleanup at all)

None run when server process receives SIGTERM from external kill.

## Implementation Strategy

This fix should be implemented as **two separate pull requests** to maximize reviewability and minimize risk:

### PR 1: Fix Process Leaks in LLM Tools SDK

**Scope**: Augment the existing Tool SDK with process tracking

**Changes**:
- Add `ProcessRegistry` for central PID tracking
- Add `ctx.spawnTracked()` to existing `Tool.Context`
- Migrate `bash` tool to use tracked spawning
- Add server signal handlers for cleanup

**Impact**: 
- Fixes bash tool process leaks (most common issue)
- Establishes foundation for server-wide cleanup
- Low risk, high value
- Easy to review and test

**Target**: Quick merge for immediate bug fix

### PR 2: Introduce Server Infrastructure SDK

**Scope**: Create new SDK for server-level services (PTY, LSP)

**Changes**:
- Create `packages/opencode/src/server/` directory structure
- Implement `createSpawnHelper()` for infrastructure
- Refactor PTY to use new Server SDK
- Refactor LSP to use new Server SDK

**Impact**:
- Fixes PTY and LSP process leaks
- Introduces plugin architecture for infrastructure
- Sets foundation for future extensibility (SSH, Docker, etc.)
- Demonstrates architectural vision

**Target**: Architectural improvement building on PR 1

**Dependencies**: PR 2 can reference ProcessRegistry from PR 1

---

## Proposed Solution

### Add Signal Handler to OpenCode Server

The server needs to register signal handlers for `SIGTERM` and `SIGINT` that:

1. Track all active tool subprocess PIDs globally
2. On signal receipt:
   - Iterate through all active tool processes
   - Call `killTree()` equivalent for each
   - Wait for graceful termination with timeout
   - Force kill remaining processes
3. Exit server process after cleanup

### Implementation Pseudocode

```typescript
// Global registry in server or tool manager
const activeToolProcesses = new Set<number>()

// Register handlers on server startup
process.on('SIGTERM', async () => {
    console.error('SIGTERM received, cleaning up tool processes...')
    await cleanupAllTools()
    process.exit(0)
})

process.on('SIGINT', async () => {
    console.error('SIGINT received, cleaning up tool processes...')
    await cleanupAllTools()
    process.exit(0)
})

// In bash tool execute()
const proc = spawn(...)
activeToolProcesses.add(proc.pid!)

// On tool completion/error
activeToolProcesses.delete(proc.pid!)

// Cleanup function
async function cleanupAllTools() {
    const pids = Array.from(activeToolProcesses)
    await Promise.all(pids.map(pid => killTreeForPid(pid)))
}
```

### Improved Approach: Server-Level Process Registry (Recommended)

A more robust solution uses a server-level registry that tracks PIDs per session. This enables **granular session cleanup** while maintaining **total server cleanup** on shutdown.

#### Benefits
- **Session-scoped cleanup**: Deleting a session kills only its processes, server stays running
- **Total cleanup on shutdown**: Server termination kills all PIDs regardless of session
- **Multi-client support**: Each client/tab can manage its own session independently
- **No persistence needed**: In-memory only, PIDs are meaningless after restart
- **Debuggable**: Can expose active PIDs via debug endpoint

#### Implementation

**1. Create Process Registry** (`packages/opencode/src/process-registry.ts`):

Central registry for tracking all spawned processes:

```typescript
class ProcessRegistry {
  // Bash tool processes: sessionID → Set of PIDs
  private bashPids = new Map<string, Set<number>>()
  
  // PTY sessions: ptyID → PID
  private ptyPids = new Map<string, number>()
  
  // LSP servers: lspID → PID (lspID = "language-workspace" key)
  private lspPids = new Map<string, number>()
  
  // === Bash Tool Methods ===
  
  registerBash(sessionID: string, pid: number) {
    if (!this.bashPids.has(sessionID)) {
      this.bashPids.set(sessionID, new Set())
    }
    this.bashPids.get(sessionID)!.add(pid)
    console.log(`[ProcessRegistry] Registered bash PID ${pid} for session ${sessionID}`)
  }
  
  unregisterBash(sessionID: string, pid: number) {
    const pids = this.bashPids.get(sessionID)
    if (pids) {
      pids.delete(pid)
      if (pids.size === 0) {
        this.bashPids.delete(sessionID)
      }
    }
    console.log(`[ProcessRegistry] Unregistered bash PID ${pid} from session ${sessionID}`)
  }
  
  // === PTY Methods ===
  
  registerPty(ptyID: string, pid: number) {
    this.ptyPids.set(ptyID, pid)
    console.log(`[ProcessRegistry] Registered PTY ${ptyID} with PID ${pid}`)
  }
  
  unregisterPty(ptyID: string) {
    this.ptyPids.delete(ptyID)
    console.log(`[ProcessRegistry] Unregistered PTY ${ptyID}`)
  }
  
  async killPty(ptyID: string) {
    const pid = this.ptyPids.get(ptyID)
    if (!pid) return
    
    console.log(`[ProcessRegistry] Killing PTY ${ptyID} (PID ${pid})`)
    await killTreeForPid(pid)
    this.ptyPids.delete(ptyID)
  }
  
  // === LSP Methods ===
  
  registerLsp(lspID: string, pid: number) {
    this.lspPids.set(lspID, pid)
    console.log(`[ProcessRegistry] Registered LSP ${lspID} with PID ${pid}`)
  }
  
  unregisterLsp(lspID: string) {
    this.lspPids.delete(lspID)
    console.log(`[ProcessRegistry] Unregistered LSP ${lspID}`)
  }
  
  async killLsp(lspID: string) {
    const pid = this.lspPids.get(lspID)
    if (!pid) return
    
    console.log(`[ProcessRegistry] Killing LSP ${lspID} (PID ${pid})`)
    await killTreeForPid(pid)
    this.lspPids.delete(lspID)
  }
  
  // === Session-level Cleanup ===
  
  async killSession(sessionID: string) {
    const pids = this.bashPids.get(sessionID)
    if (!pids || pids.size === 0) return
    
    console.log(`[ProcessRegistry] Killing ${pids.size} bash processes for session ${sessionID}`)
    await Promise.all(Array.from(pids).map(pid => killTreeForPid(pid)))
    this.bashPids.delete(sessionID)
  }
  
  // === Global Cleanup (Server Shutdown) ===
  
  async killAll() {
    const bashPids = Array.from(this.bashPids.values()).flatMap(set => Array.from(set))
    const ptyPids = Array.from(this.ptyPids.values())
    const lspPids = Array.from(this.lspPids.values())
    
    const allPids = [...bashPids, ...ptyPids, ...lspPids]
    
    if (allPids.length === 0) return
    
    console.log(`[ProcessRegistry] Killing all processes: ${bashPids.length} bash, ${ptyPids.length} PTY, ${lspPids.length} LSP`)
    await Promise.all(allPids.map(pid => killTreeForPid(pid)))
    
    this.bashPids.clear()
    this.ptyPids.clear()
    this.lspPids.clear()
  }
  
  // === Debug Helpers ===
  
  getActiveProcesses() {
    return {
      bash: Object.fromEntries(
        Array.from(this.bashPids.entries()).map(([sid, pids]) => [sid, Array.from(pids)])
      ),
      pty: Object.fromEntries(this.ptyPids.entries()),
      lsp: Object.fromEntries(this.lspPids.entries()),
    }
  }
}

export const processRegistry = new ProcessRegistry()
```

**2. Create Server SDK** (`packages/opencode/src/server/spawn.ts` - NEW FILE):

Unified spawn helper for all server infrastructure (PTY, LSP, and backing for Tool SDK):

```typescript
import { spawn, type ChildProcessWithoutNullStreams, type SpawnOptions } from 'child_process'
import { processRegistry } from '../process-registry'

export interface SpawnContext {
  scopeID: string       // sessionID, ptyID, lspID, etc.
  scopeType: 'bash' | 'pty' | 'lsp'
  abort?: AbortSignal   // Optional cancellation
}

/**
 * Server SDK spawn helper with automatic process tracking.
 * Used by PTY, LSP, and Tool SDK.
 */
export function createSpawnHelper(ctx: SpawnContext) {
  return (command: string, args: string[], options: SpawnOptions): ChildProcessWithoutNullStreams => {
    const proc = spawn(command, args, options)
    const pid = proc.pid!
    
    // Auto-register with process registry
    if (ctx.scopeType === 'bash') {
      processRegistry.registerBash(ctx.scopeID, pid)
    } else if (ctx.scopeType === 'pty') {
      processRegistry.registerPty(ctx.scopeID, pid)
    } else if (ctx.scopeType === 'lsp') {
      processRegistry.registerLsp(ctx.scopeID, pid)
    }
    
    // Auto-unregister on process exit
    proc.once('exit', () => {
      if (ctx.scopeType === 'bash') {
        processRegistry.unregisterBash(ctx.scopeID, pid)
      } else if (ctx.scopeType === 'pty') {
        processRegistry.unregisterPty(ctx.scopeID)
      } else if (ctx.scopeType === 'lsp') {
        processRegistry.unregisterLsp(ctx.scopeID)
      }
    })
    
    // Auto-unregister on abort signal (if provided)
    if (ctx.abort) {
      ctx.abort.addEventListener('abort', () => {
        if (ctx.scopeType === 'bash') {
          processRegistry.unregisterBash(ctx.scopeID, pid)
        } else if (ctx.scopeType === 'pty') {
          processRegistry.unregisterPty(ctx.scopeID)
        } else if (ctx.scopeType === 'lsp') {
          processRegistry.unregisterLsp(ctx.scopeID)
        }
      }, { once: true })
    }
    
    return proc
  }
}
```

**3. Integrate with Tool SDK** (`packages/opencode/src/tool/tool.ts`):

Tool SDK uses Server SDK under the hood:

```typescript
import { createSpawnHelper } from '../server/spawn'
import type { ChildProcessWithoutNullStreams, SpawnOptions } from 'child_process'

export type Context<M extends Metadata = Metadata> = {
  sessionID: string
  messageID: string
  agent: string
  abort: AbortSignal
  callID?: string
  extra?: { [key: string]: any }
  metadata(input: { title?: string; metadata?: M }): void
  
  // NEW: Tracked process spawning for tools
  spawnTracked(command: string, args: string[], options: SpawnOptions): ChildProcessWithoutNullStreams
}
```

Implement in `Tool.define()` (line ~48):

```typescript
toolInfo.execute = (args, ctx) => {
  // ... existing validation ...
  
  // Augment context with Server SDK spawn helper
  const spawnHelper = createSpawnHelper({
    scopeID: ctx.sessionID,
    scopeType: 'bash',
    abort: ctx.abort,
  })
  
  const augmentedCtx: Context = {
    ...ctx,
    spawnTracked: spawnHelper,
  }
  
  return execute(args, augmentedCtx)
}
```

**4. Migrate Bash Tool** (`packages/opencode/src/tool/bash.ts`):

Replace raw `spawn()` with `ctx.spawnTracked()`:

```typescript
// OLD (line ~216):
const proc = spawn(params.command, {
  shell,
  cwd: Instance.directory,
  // ...
})

// NEW:
const proc = ctx.spawnTracked(params.command, {
  shell,
  cwd: Instance.directory,
  // ...
})

// No manual register/unregister needed - SDK handles it!
```

**Benefits**:
- ✅ All tools automatically get process tracking
- ✅ No manual register/unregister in tool code
- ✅ Future tools that spawn processes "just work"
- ✅ Uses Server SDK for consistency

**5. Integrate with PTY** (`packages/opencode/src/pty/index.ts`):

PTY uses Server SDK for spawn management:

```typescript
import { createSpawnHelper } from '../server/spawn'

// In create() function (line ~112)
export async function create(input: CreateInput) {
  const id = Identifier.create("pty", false)
  const command = input.command || shell()
  const args = input.args || []
  const cwd = input.cwd || Instance.directory
  const env = { ...process.env, ...input.env } as Record<string, string>
  
  log.info("creating session", { id, cmd: command, args, cwd })
  
  // Use Server SDK spawn helper
  const spawnHelper = createSpawnHelper({
    scopeID: id,
    scopeType: 'pty',
  })
  
  const spawn = await pty()
  const ptyProcess = spawnHelper(command, args, {
    name: "xterm-256color",
    cwd,
    env,
  })
  
  // ... rest of PTY setup
  // Note: exit handler and cleanup are now automatic via Server SDK
}

// In remove() when killing PTY (line ~176)
export async function remove(id: string) {
  const session = state().get(id)
  if (!session) return
  
  log.info("removing session", { id })
  
  // Kill via registry to ensure cleanup
  await processRegistry.killPty(id)
  
  // ... rest of cleanup
}
```

**6. Integrate with LSP** (`packages/opencode/src/lsp/server.ts` and `src/lsp/index.ts`):

LSP uses Server SDK for spawn management:

```typescript
import { createSpawnHelper } from '../server/spawn'

// In each LSP spawn function (e.g., Typescript.spawn at line ~90)
export const Typescript: Info = {
  id: "typescript",
  root: NearestRoot([...]),
  extensions: [".ts", ".tsx", ...],
  async spawn(root) {
    const tsserver = await Bun.resolve("typescript/lib/tsserver.js", Instance.directory).catch(() => {})
    if (!tsserver) return
    
    const lspID = `typescript-${root}`
    
    // Use Server SDK spawn helper
    const spawnHelper = createSpawnHelper({
      scopeID: lspID,
      scopeType: 'lsp',
    })
    
    const proc = spawnHelper(
      BunProc.which(),
      ["x", "typescript-language-server", "--stdio"],
      {
        cwd: root,
        env: {
          ...process.env,
          BUN_BE_BUN: "1",
        },
      }
    )
    
    // Exit and cleanup are automatic via Server SDK
    return {
      process: proc,
      lspID,
      initialization: {
        tsserver: { path: tsserver },
      },
    }
  },
}

// Similar updates for Deno, Vue, ESLint LSP spawn functions...

// When stopping LSP server (in LSP manager):
export async function stopServer(lspID: string) {
  await processRegistry.killLsp(lspID)
  // ... rest of cleanup
}
```

**7. Session Deletion** (wherever DELETE /session is handled):

```typescript
import { processRegistry } from './process-registry'

async function deleteSession(sessionID: string) {
  // Kill active tool processes for this session
  await processRegistry.killSession(sessionID)
  
  // Then delete session files, etc.
  // ...
}
```

**8. Server Signal Handlers** (`packages/opencode/src/index.ts`):

```typescript
import { processRegistry } from './process-registry'

process.on('SIGTERM', async () => {
  console.error('SIGTERM received, cleaning up all processes...')
  await processRegistry.killAll()
  process.exit(0)
})

process.on('SIGINT', async () => {
  console.error('SIGINT received, cleaning up all processes...')
  await processRegistry.killAll()
  process.exit(0)
})
```

**9. Optional Debug Endpoint**:

```typescript
// In server routes
app.get('/debug/processes', (req, res) => {
  res.json(processRegistry.getActiveProcesses())
})

// Example response:
// {
//   "bash": {
//     "session_abc123": [54321, 54322]
//   },
//   "pty": {
//     "pty_xyz789": 54400
//   },
//   "lsp": {
//     "typescript-/path/to/project": 54500,
//     "deno-/path/to/project": 54501
//   }
// }
```

#### Usage Scenarios

**Scenario 1: User closes single tab in EGUI**
```
1. EGUI sends DELETE /session/{id}
2. Server calls processRegistry.killSession(id)
3. Only that session's tool processes are terminated
4. Server continues running, other sessions unaffected
```

**Scenario 2: User closes entire EGUI application**
```
1. EGUI sends SIGTERM to server
2. Server signal handler calls processRegistry.killAll()
3. All tool processes across all sessions terminated
4. Server exits cleanly
```

**Scenario 3: Tool completes naturally**
```
1. Bash tool process exits
2. proc.once('exit') handler fires
3. processRegistry.unregisterBash() removes PID
4. No cleanup needed, already complete
```

**Scenario 4: User closes PTY terminal**
```
1. EGUI sends DELETE /pty/{id}
2. Server calls Pty.remove(id)
3. processRegistry.killPty(id) terminates shell
4. PTY removed from registry
5. Server continues running
```

**Scenario 5: LSP server no longer needed**
```
1. Last file of a language/workspace closes
2. LSP manager calls stopServer(lspID)
3. processRegistry.killLsp(lspID) terminates LSP
4. LSP removed from registry
5. Server continues running
```

### Implementation Locations

#### PR 1: LLM Tools SDK (Existing SDK Enhancement)

**Core Infrastructure**:
1. **Process Registry**: `packages/opencode/src/process-registry.ts` - **NEW FILE** - Central PID tracking
2. **Server Signal Handlers**: `packages/opencode/src/index.ts` - Add SIGTERM/SIGINT handlers

**Tool SDK Enhancement**:
3. **Tool SDK**: `packages/opencode/src/tool/tool.ts` - Add `ctx.spawnTracked()` to existing SDK
4. **Bash tool**: `packages/opencode/src/tool/bash.ts:216` - Migrate to use `ctx.spawnTracked()`

**API Integration**:
5. **Session deletion**: Wherever `DELETE /session/{id}` is handled - Call `processRegistry.killSession()`
6. **Cleanup utility**: Extract `killTree()` from bash.ts into shared utility

**Files Changed**: ~4-5 files  
**Lines of Code**: ~300-400 LOC  
**Risk Level**: Low (only touches Tool SDK)

---

#### PR 2: Server Infrastructure SDK (New Architecture)

**New SDK Creation**:
1. **Server SDK**: `packages/opencode/src/server/spawn.ts` - **NEW FILE** - `createSpawnHelper()` for infrastructure
2. **Server Plugin Types**: `packages/opencode/src/server/types.ts` - **NEW FILE** - Type definitions (optional)

**Infrastructure Refactoring**:
3. **PTY**: `packages/opencode/src/pty/index.ts:121` - Refactor to use `createSpawnHelper()`
4. **LSP**: `packages/opencode/src/lsp/server.ts` - All spawn functions refactored to use `createSpawnHelper()`

**Process Registry Integration**:
5. **Update ProcessRegistry**: Add PTY and LSP methods (building on PR 1)

**Optional Tool SDK Refactoring**:
6. **Tool SDK**: Optionally refactor `ctx.spawnTracked()` to use Server SDK (DRY improvement)

**Files Changed**: ~6-8 files  
**Lines of Code**: ~400-600 LOC  
**Risk Level**: Medium (touches core infrastructure)  
**Depends On**: PR 1 (uses ProcessRegistry)

## Verification Steps

### PR 1 Testing: LLM Tools SDK

Verify bash tool process cleanup:

**Test 1: Bash Tool Cleanup**
```bash
# 1. Start EGUI client → spawns OpenCode server
# 2. Execute long-running bash tool: sleep 300
# 3. Close EGUI client while tool is running
# 4. Verify:
ps aux | grep -i opencode
ps aux | grep -i "sleep 300"

# Should show NO orphaned processes
```

**Test 2: Session Deletion**
```bash
# 1. Start two sessions with different bash tools
# 2. Session A: sleep 300
# 3. Session B: sleep 400
# 4. Delete only session A via DELETE /session/{A}
# 5. Verify:
ps aux | grep sleep

# Should show sleep 400 still running (session B)
# Should NOT show sleep 300 (session A cleaned up)
```

**Test 3: Debug Endpoint**
```bash
# While server running with active bash tool:
curl http://localhost:PORT/debug/processes

# Should show bash PIDs by session
```

---

### PR 2 Testing: Server Infrastructure SDK

Verify PTY and LSP process cleanup:

**Test 1: PTY Session Cleanup
```bash
# 1. Start EGUI client
# 2. Open terminal (PTY) via API or UI
# 3. Start long process in terminal: tail -f /var/log/system.log
# 4. Close EGUI client
# 5. Verify:
ps aux | grep -i tail
ps aux | grep -E 'zsh|bash'  # Check for orphaned shells

# Should show NO orphaned shell or tail processes
```

**Test 2: LSP Server Cleanup
```bash
# 1. Start EGUI client
# 2. Open TypeScript file → triggers typescript-language-server
# 3. Note LSP PID: ps aux | grep typescript-language-server
# 4. Close EGUI client
# 5. Verify:
ps aux | grep typescript-language-server
ps aux | grep -E 'deno.*lsp|vue-language-server|eslint'

# Should show NO orphaned LSP processes
```

**Test 3: Combined Cleanup**
```bash
# With all subsystems active:
# - Bash tool: sleep 300
# - PTY session: tail -f /var/log/system.log
# - LSP: typescript-language-server

# Close EGUI client
# Verify ALL are cleaned up:
ps aux | grep -E 'sleep|tail|typescript-language-server'

# Should show NO orphaned processes from any subsystem
```

## Impact Assessment

### Without Fix
- Process leaks accumulate with heavy tool usage
- Port conflicts from leaked dev servers
- Resource exhaustion on long-running sessions
- User confusion about "zombie" processes

### With Fix
- Clean shutdown semantics
- Predictable resource management
- Better user experience
- Prevents accidental DoS from process accumulation

## Priority Justification

**PR 1** is HIGH priority because:
- Fixes most common leak (bash tools)
- Affects core agent functionality
- Simple to trigger (close client during tool call)
- Low risk, high value fix

**PR 2** is MEDIUM-HIGH priority because:
- Fixes remaining leaks (PTY, LSP)
- Introduces architectural improvement
- Builds foundation for future extensibility
- Slightly higher risk (touches infrastructure)

## Architecture Benefits

**Layered SDK Design**:

```
┌─────────────────────────────────────────┐
│   LLM Tools (agent-callable)           │
│   bash, grep, read, write               │
│   Uses: ctx.spawnTracked()             │
└─────────────────────────────────────────┘
                 ↓
┌─────────────────────────────────────────┐
│   Tool SDK                              │
│   packages/opencode/src/tool/tool.ts   │
│   Provides: Tool.Context                │
└─────────────────────────────────────────┘
                 ↓
┌─────────────────────────────────────────┐
│   Server SDK (NEW)                      │
│   packages/opencode/src/server/spawn.ts│
│   Provides: createSpawnHelper()         │
│   Used by: Tool SDK, PTY, LSP           │
└─────────────────────────────────────────┘
                 ↓
┌─────────────────────────────────────────┐
│   Process Registry                      │
│   Central PID tracking                  │
└─────────────────────────────────────────┘
```

**Key Benefits**:

1. **Unified Interface**: All process spawning goes through Server SDK
   - Tools use `ctx.spawnTracked()` (which wraps Server SDK)
   - PTY uses `createSpawnHelper()` directly
   - LSP uses `createSpawnHelper()` directly

2. **Separation of Concerns**:
   - Tool SDK: Agent-facing API for LLM tools
   - Server SDK: Infrastructure-facing API for subsystems
   - Process Registry: Low-level PID tracking

3. **No Direct Registry Access**:
   - No subsystem directly imports ProcessRegistry
   - All access mediated through Server SDK
   - Easier to add features (metrics, rate limiting, etc.)

4. **Future-Proof**:
   - New tools automatically get tracking
   - New subsystems (future SSH, Docker, etc.) use same pattern
   - Consistent spawn management across entire codebase

5. **Maintainability**:
   - Single source of truth for spawn logic
   - Easy to add logging, metrics, resource limits
   - Changes propagate automatically to all consumers

## Related Files

### Core Implementation
- `packages/opencode/src/process-registry.ts` - **New file** for process tracking
- `packages/opencode/src/server/spawn.ts` - **New file** for Server SDK
- `packages/opencode/src/tool/tool.ts` - Tool SDK enhancement
- `packages/opencode/src/index.ts` - Server entry point for signal handlers

### Subsystem Integration
- `packages/opencode/src/tool/bash.ts` - Bash tool execution
- `packages/opencode/src/pty/index.ts` - PTY terminal session management
- `packages/opencode/src/lsp/server.ts` - LSP server spawn functions
- `packages/opencode/src/lsp/index.ts` - LSP lifecycle management

### Client Side
- `clients/egui/src/app.rs` - Client cleanup on exit
- `clients/egui/src/discovery/process.rs` - Server process management

### API Endpoints
- Session deletion endpoint - Integrate session cleanup
- PTY endpoints - Integrate PTY cleanup
- Debug endpoint - Expose process registry state
