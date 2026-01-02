# QA Task: Session 1 - Shared Rust Core with Production-Grade Error Handling

## Files Under Test

**Created in this session:**

- `backend/client-core/src/lib.rs` - Public API (constants only)
- `backend/client-core/src/error/mod.rs` - Top-level error enum (`CoreError`)
- `backend/client-core/src/error/discovery.rs` - Discovery-specific errors (`DiscoveryError`)
- `backend/client-core/src/error/spawn.rs` - Spawn-specific errors (`SpawnError`)
- `backend/client-core/src/discovery/mod.rs` - Discovery module exports
- `backend/client-core/src/discovery/process.rs` - Server discovery, stop, health check
- `backend/client-core/src/discovery/spawn.rs` - Server spawn logic with health wait
- `common/src/error/error_location.rs` - Error location tracking utility

## Code Summary

Implemented a production-grade Rust core for discovering and spawning OpenCode servers. The discovery module finds running servers via process scanning and network socket inspection. The spawn module launches new server processes, parses their stdout for the listening URL, and waits for health checks. All errors include file/line/column location tracking for debugging. Uses exponential backoff for retries (health checks, process kill verification).

## High-Risk Areas (Focus Testing Here)

### 1. Process Discovery Logic (`discovery/process.rs`)

**Why risky:** Complex process scanning + network socket correlation. Race conditions possible (process dies between scan and inspection). Multiple failure modes (network query fails, process disappears, port not found).

**Test requirements:**

- **Error handling:** Test `discover()` when network query fails (mock `get_sockets_info`)
- **Error handling:** Test `discover_on_port()` when process disappears between socket scan and `with_process()` call
- **Edge cases:** Empty process list, no listening sockets, multiple OpenCode processes
- **Skip:** Don't test `sysinfo` or `netstat2` libraries themselves - trust their implementations
- **Focus on YOUR code:** Test that discovery correctly correlates PIDs with ports, handles missing processes gracefully

### 2. Server Spawn and URL Parsing (`discovery/spawn.rs`)

**Why risky:** Parsing stdout from external process. Regex matching. Multiple async operations. Process lifecycle management (cleanup on failure, detach on success). Timeout scenarios.

**Test requirements:**

- **Error handling:** Test `spawn_and_wait()` when opencode binary not found (both PATH and local)
- **Error handling:** Test `parse_server_url()` when:
  - Server outputs no URL in first 100 lines
  - Server outputs malformed URL (regex doesn't match)
  - Server outputs wrong hostname (not 127.0.0.1)
  - Port parsing fails (non-numeric port)
- **Error handling:** Test `wait_for_health()` timeout scenario (server never becomes healthy)
- **Business logic:** Test that process is killed on health check failure
- **Business logic:** Test that process is detached (forgotten) on success
- **Edge cases:** Server process exits before printing URL
- **Skip:** Don't test tokio process spawning itself - test YOUR error handling around it

### 3. Process Termination (`discovery/process.rs::stop_pid`)

**Why risky:** OS signal handling. Exponential backoff verification loop. Process might not die, might already be dead.

**Test requirements:**

- **Business logic:** Test `stop_pid()` with non-existent PID (should return false)
- **Business logic:** Test SIGTERM path vs SIGKILL fallback (mock process kill behavior)
- **Timeout scenario:** Test process that refuses to die (backoff exhausted)
- **Skip:** Don't test OS signal delivery - test YOUR retry logic and return values

### 4. Error Location Tracking (`common/error_location.rs`)

**Why risky:** Low complexity but critical for debugging. Must propagate correctly.

**Test requirements:**

- **Business logic:** Test `ErrorLocation::from()` captures correct file/line/column
- **Display format:** Test `Display` implementation produces expected format `[file:line:column]`
- **Integration:** Verify errors in discovery/spawn modules actually contain location info
- **Skip:** This is simple enough to verify manually, low priority for unit tests

## External Dependencies (Test YOUR Integration Only)

**Do NOT test these dependencies themselves. Test YOUR error handling.**

- **`sysinfo` crate (process scanning):**
  - Test: How does `with_process()` handle process not found? (returns `None` - verify this)
  - Test: Does discovery gracefully handle empty process list?
- **`netstat2` crate (network socket inspection):**
  - Test: How does `query_tcp_sockets()` wrap errors into `DiscoveryError::NetworkQuery`?
  - Test: Does discovery handle no listening sockets scenario?

- **`tokio::process` (process spawning):**
  - Test: How does spawn handle `ErrorKind::NotFound`? (falls back to local binary)
  - Test: Does spawn propagate IO errors into `SpawnError::Spawn`?

- **`reqwest` (health checks):**
  - Test: Does `check_health()` return false on timeout/connection refused?
  - Skip: Don't test reqwest's HTTP client - test YOUR boolean return logic

- **`backoff` crate (exponential backoff):**
  - Test: Does `wait_for_health()` respect max elapsed time?
  - Test: Does `stop_pid()` verification loop terminate?
  - Skip: Don't test backoff algorithm - test YOUR usage of it

## Relevant Code Snippets

### Critical Function: discover()

```rust
pub fn discover() -> Result<Option<ServerInfo>, DiscoveryError> {
    if let Some(override_port) = get_override_port() {
        return discover_on_port(override_port);
    }
    discover_by_process_scan()
}
```

**Why critical:** Entry point for server discovery. Branches on port override.

### Critical Function: spawn_and_wait()

```rust
pub async fn spawn_and_wait() -> Result<ServerInfo, SpawnError> {
    let port_arg = get_override_port()
        .map(|p| p.to_string())
        .unwrap_or_else(|| AUTO_SELECT_PORT.to_string());

    let child = spawn_server_process(&port_arg).await?;
    let (mut child, base_url, port) = parse_server_url(child).await?;

    if let Err(e) = wait_for_health(&base_url).await {
        let _ = child.kill().await;  // CRITICAL: cleanup on failure
        return Err(e);
    }

    forget(child);  // CRITICAL: detach on success
    Ok(ServerInfo { /* ... */ owned: true })
}
```

**Why critical:** Complex error handling. Must kill process on failure. Must detach on success.

### Critical Function: parse_server_url()

```rust
async fn parse_server_url(mut child: TokioChild) -> Result<(TokioChild, String, u16), SpawnError> {
    let stdout = child.stdout.take().ok_or_else(/* error */)?;
    let mut lines = BufReader::new(stdout).lines();
    let re = get_url_regex();

    for _ in 0..SPAWN_MAX_OUTPUT_LINES {
        match lines.next_line().await {
            Ok(Some(line)) => {
                if let Some(cap) = re.captures(&line) {
                    let host = cap.name("host")?.as_str();
                    let port = cap.name("port")?.as_str().parse::<u16>()?;

                    if host != OPENCODE_SERVER_HOSTNAME {
                        warn!("Unexpected hostname: {host}");
                    }

                    return Ok((child, base_url, port));
                }
            }
            Ok(None) => break,
            Err(e) => return Err(/* parse error */),
        }
    }
    Err(SpawnError::Parse { /* no URL found */ })
}
```

**Why critical:** Parsing external process output. Multiple failure modes. Regex matching.

### Critical Function: stop_pid()

```rust
pub fn stop_pid(pid: u32) -> bool {
    let killed = with_process(pid, |p| {
        p.kill_with(Signal::Term).unwrap_or_else(|| p.kill())
    }).unwrap_or(false);

    if !killed {
        return false;
    }

    // Exponential backoff to verify termination
    let mut backoff = ExponentialBackoff { max_elapsed_time: Some(5s), .. };

    loop {
        if with_process(pid, |_| true).is_none() {
            return true;  // Process terminated
        }

        match backoff.next_backoff() {
            Some(duration) => sleep(duration),
            None => return false,  // Timeout
        }
    }
}
```

**Why critical:** Retry logic with timeout. Must verify process actually died.

## Value Goals (NOT Coverage Goals)

- **Critical paths:** 100% coverage for error handling, process lifecycle, parsing logic
- **Focus areas:**
  1. Error propagation (all error paths return correct error types with locations)
  2. Process cleanup (spawn failures kill child process)
  3. Retry logic (exponential backoff respects timeouts)
  4. URL parsing edge cases (malformed output, wrong hostname, no URL)
  5. Process termination verification (stop_pid actually confirms death)
- **Skip:**
  - Simple getters (e.g., `ServerInfo` field access)
  - Constant definitions (e.g., `OPENCODE_BINARY`)
  - Dependency functionality (sysinfo, netstat2, tokio, reqwest)
  - OnceLock initialization (trivial wrapper)

- **Target:** Tests that catch real bugs:
  - Process leaks (spawn fails but doesn't kill child)
  - Deadlocks (backoff never terminates)
  - Silent failures (errors swallowed)
  - Incorrect error location tracking
  - URL parsing regressions

## Success Criteria

- [x] High-risk areas have comprehensive tests (error handling, process lifecycle, parsing)
- [x] Error types contain correct `ErrorLocation` tracking
- [x] Process cleanup verified (spawn failures don't leak processes)
- [x] Exponential backoff timeouts respected
- [x] URL parsing handles malformed input gracefully
- [x] Low-value tests skipped with reasoning
- [x] All tests pass
- [x] Tests follow Given-When-Then structure (both naming and internal organization)
- [x] All tests documented with value proposition (VALUE, WHY THIS MATTERS, BUG THIS CATCHES)

**Status:** ✅ Complete - See `QA_RESULTS_SESSION_1.md` for detailed results

## Testing Constraints

- Follow existing Rust test patterns (discover by searching for `#[cfg(test)]` modules)
- Use standard Rust test framework (`#[test]`, `assert_eq!`, etc.)
- Skip testing dependency implementations (trust sysinfo, tokio, reqwest)
- Don't test trivial code (constants, simple getters)
- Focus on bug prevention, not coverage %
- Mock external dependencies (processes, network calls) to test error paths

## Mock Strategy

**Recommended mocking approach:**

1. **Process operations:** Difficult to mock `sysinfo::Process` directly. Instead:
   - Test with "process not found" path (pass invalid PID)
   - Integration test with real processes (if feasible in CI)
   - Focus on testing logic AROUND process operations

2. **Network operations:** Mock `get_sockets_info()` results
   - Create test helper that returns empty list, populated list, etc.
   - Test discovery correlation logic without real network state

3. **Process spawning:** Mock difficult, focus on error handling
   - Test `spawn_local_binary()` path with invalid exe path
   - Test `spawn_server_process()` fallback from PATH to local
   - Integration test spawn with real `opencode` binary (if available)

4. **Health checks:** Mock HTTP client or test with test server
   - Create test HTTP server that returns 200 / 500 / timeout
   - Or test `check_health()` against real endpoint in integration test

**Pragmatic recommendation:**

- Unit test error handling paths (easy to trigger)
- Unit test parsing logic (feed mock stdout lines)
- Integration test happy paths (requires real opencode binary)
- Skip mocking complex OS interactions (diminishing returns)

## Additional Context

**Session 1 Accomplishments:**

- This is the foundation for Tauri-Blazor desktop client
- Code is production-grade: zero magic numbers, DRY, full rustdoc, clippy clean
- All errors use `ErrorLocation` for debugging
- Exponential backoff used consistently (health checks, kill verification)
- Regex compiled once with `OnceLock` for performance
- Process cleanup on all failure paths
- No changes to egui client (remains independent)

**What's NOT included:**

- No Tauri integration yet (Session 2)
- No Blazor frontend yet (Session 3)
- No authentication yet (Session 5)

**Known design decisions:**

- Discovery only finds localhost servers (intentional security boundary)
- Spawn detaches process on success (daemon model)
- Health check timeout is 20 seconds (may be too long, but safe)
- URL parsing limited to first 100 lines of output (reasonable assumption)

---

**Run this task in an isolated session. All context needed is above.**
