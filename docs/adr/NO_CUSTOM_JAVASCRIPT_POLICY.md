# OpenCode: No Custom JavaScript Policy

**Status:** Universal Constraint  
**Applies to:** All ADRs, all frontend implementations  
**Date:** 2026-01-02  
**Authority:** Repository owner (Tony)

---

## Policy Statement

**OpenCode has a ZERO CUSTOM JAVASCRIPT policy.**

Any architectural decision involving frontend technology **MUST NOT** require writing or maintaining hand-written JavaScript files.

## What This Means

### ❌ NOT ALLOWED

- Hand-written `.js` files in `wwwroot/js/` or similar directories
- Custom JavaScript helper functions or utilities
- JavaScript event handlers written manually
- JavaScript for DOM manipulation
- JavaScript wrappers around library APIs
- Any `.js` file that requires human editing or maintenance

### ✅ ALLOWED

- **Machine-generated JavaScript** produced by framework compilers:
  - Blazor WASM's `_framework/blazor.webassembly.js`
  - TypeScript compiler output (if TypeScript is framework-managed)
  - Build tool outputs (Vite, webpack, etc.) that are fully automated
  - Framework-bundled JavaScript (React, Vue, Svelte compiled output)

- **Third-party library JavaScript** (bundled, unmodified):
  - Tauri's bundled `__TAURI__` APIs
  - UI component libraries (Radzen, MudBlazor) that ship with JS
  - CDN-loaded libraries (as long as no custom wrappers needed)

- **Framework-internal JavaScript**:
  - SolidJS's compiled output
  - React/Vue compiler output
  - Any framework where **WE ONLY WRITE IN THE FRAMEWORK'S LANGUAGE** (JSX, .razor, .vue, etc.)

## Rationale

1. **Maintenance burden**: JavaScript requires constant vigilance for:
   - Browser compatibility issues
   - Security vulnerabilities in dependencies
   - Type safety problems (even with TypeScript)
   - Async/promise handling bugs
   - Event handling edge cases

2. **Context switching**: Maintaining multiple languages (Rust + C#/JSX/etc. + JavaScript) is cognitive overhead

3. **Framework reliability**: Modern frameworks (Blazor, SolidJS, React) handle all DOM/event logic internally - we don't need custom JS

4. **Type safety**: Rust and C# are type-safe. JavaScript is not. Eliminating custom JS reduces entire classes of bugs.

5. **Build complexity**: Custom JS often requires transpilation, bundling, minification - more moving parts

## Implications for Architecture Decisions

### Tauri + Blazor (ADR-0001)

✅ **COMPLIANT** - Blazor generates all JavaScript automatically

- C# components compile to WASM
- Blazor runtime handles DOM updates via generated JS
- Tauri IPC via C# `IJSRuntime.InvokeAsync("window.__TAURI__.core.invoke", ...)`
- No custom JS files needed

**Example (CORRECT):**

```csharp
// C# service calling Tauri - NO CUSTOM JS
public async Task<ServerInfo> SpawnServerAsync()
{
    return await _jsRuntime.InvokeAsync<ServerInfo>(
        "window.__TAURI__.core.invoke",
        "spawn_server"
    );
}
```

**Example (FORBIDDEN):**

```javascript
// wwwroot/js/tauri-helper.js - DO NOT CREATE THIS
export async function spawnServer() {
  return await window.__TAURI__.core.invoke("spawn_server")
}
```

### Existing packages/tauri (SolidJS)

✅ **COMPLIANT** - SolidJS compiles JSX to JavaScript

- Developers write `.tsx` files (JSX/TypeScript)
- Vite/SolidJS compiler generates JavaScript
- No hand-written `.js` files in the codebase

### Electron (if considered)

❌ **PROBLEMATIC** - Electron typically requires:

- Main process JavaScript (renderer/main communication)
- Preload scripts (security boundary)
- Custom IPC handlers

**Verdict:** Electron would require custom JS, making it **incompatible** with this policy unless all logic is in C#/Rust with Electron.NET (which has other issues)

### Pure Web (if considered)

✅ **CONDITIONALLY COMPLIANT**

- If using framework: React/Vue/Svelte/SolidJS → OK (JSX/TSX only)
- If using vanilla HTML+JS → ❌ NOT ALLOWED
- Must use a framework that compiles to JS, so we never touch `.js` directly

## How to Validate Compliance

For any ADR or implementation, check:

1. **File extension audit**: `find . -name "*.js" -not -path "*/node_modules/*" -not -path "*/dist/*" -not -path "*/_framework/*"`
   - Should return ZERO results (except build outputs)

2. **Source code check**: Are developers expected to write/edit `.js` files?
   - If YES → ❌ REJECT
   - If NO, only framework files (.razor, .tsx, .vue) → ✅ ACCEPT

3. **Framework abstraction**: Does the framework handle all JavaScript generation?
   - Blazor → YES ✅
   - SolidJS/React/Vue → YES ✅
   - Vanilla JS → NO ❌
   - jQuery-based → NO ❌

## Exceptions

There are **NO EXCEPTIONS** to this policy.

If a feature absolutely requires custom JavaScript and cannot be implemented any other way, **the feature is out of scope** for OpenCode or must wait for a framework solution.

## Examples from Cognexus

**Cognexus has `wwwroot/js/renderer-helper.js` - Is this a violation?**

**Context:** Cognexus is a GPU-accelerated node graph editor using WGPU for rendering. The custom JS is a thin wrapper to initialize the WebGPU/WebGL canvas.

**Analysis:**

- For **Cognexus specifically**: Acceptable - it's a specialized graphics application
- For **OpenCode**: NOT APPLICABLE - OpenCode is a chat/CRUD app, no GPU rendering
- **Lesson**: Even Cognexus minimizes custom JS (12 lines total, just WASM init)

**Conclusion for OpenCode:**

- Cognexus's pattern does NOT transfer to OpenCode
- OpenCode needs ZERO custom JavaScript (not even 12 lines)
- Standard Blazor components (Radzen) handle all rendering

## Enforcement

This policy is enforced via:

1. **ADR review**: All ADRs checked for custom JS requirements
2. **Code review**: PRs with `.js` files (outside build outputs) rejected
3. **CI/CD** (future): Lint check to ensure no custom JS in source tree

## Related Decisions

- [ADR-0001: Tauri + Blazor Desktop Client](0001-tauri-blazor-desktop-client.md) - Explicitly designed around this constraint
- [Template ADR](template.md) - Lists this as universal constraint

## Updates

- **2026-01-02**: Initial policy document created
