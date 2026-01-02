# Architecture Decision Records (ADRs)

This directory contains Architecture Decision Records for the OpenCode project.

## What is an ADR?

An Architecture Decision Record (ADR) captures an important architectural decision made along with its context and consequences. ADRs provide a historical record of why certain decisions were made.

## Universal Constraints

All ADRs in this repository must comply with:

- **[No Custom JavaScript Policy](NO_CUSTOM_JAVASCRIPT_POLICY.md)** - Zero hand-written `.js` files. Only machine-generated JavaScript allowed.

## Universal Policies

- **[No Custom JavaScript Policy](NO_CUSTOM_JAVASCRIPT_POLICY.md)** - Hard requirement for all frontend decisions

## ADR Index

<!-- Each ADR should be listed here in ascending order -->

- [ADR-0001: Add Tauri + Blazor WebAssembly Desktop Client](0001-tauri-blazor-desktop-client.md) - **Proposed**

---

## Status Definitions

- **Proposed** - Decision is under discussion
- **Accepted** - Decision has been agreed upon and is being/has been implemented
- **Deprecated** - Decision is no longer recommended but may still be in use
- **Superseded by ADR-XXXX** - Decision has been replaced by a newer ADR

## Creating a New ADR

1. Copy `template.md` to a new file `NNNN-short-title.md` (use next sequential number)
2. Fill in all sections
3. Submit for review
4. Update this README with the new ADR entry
