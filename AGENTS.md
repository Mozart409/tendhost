# AGENTS.md - Coding Guidelines for Agentic Development

This file provides guidelines for AI agents and automated tools working in the tendhost repository.

> **Last Updated**: 2026-02-19T22:29:50+01:00  
> **Commit**: b4ae6d8fa38cecda9e77c3a8b360458ed28551bd

## Project Context

**IMPORTANT**: Before working on any code, read [GOALS.md](./GOALS.md) to understand:
- The project's vision and architecture
- Actor model design (`OrchestratorActor`, `HostActor`)
- State machine transitions for host lifecycle
- Workspace structure and crate responsibilities
- Core traits (`RemoteExecutor`, `PackageManager`)
- API design (REST + WebSocket)

GOALS.md defines the **what** and **why** of this project; AGENTS.md defines the **how**.

## Build, Lint, and Test Commands

### Quick Checks
```bash
# Check compilation (fast, default job in bacon)
cargo check
cargo check --all-targets  # includes tests and examples

# Format check
cargo fmt --check --all

# Clippy with warnings
cargo clippy --all-targets -- -D warnings

# Pedantic clippy (strict, all lints)
cargo clippy --all-targets -- -W clippy::pedantic -W clippy::correctness -W clippy::suspicious -W clippy::complexity -W clippy::perf -D warnings
```

### Testing
```bash
# Run all tests
cargo test --all

# Run a single test
cargo test --lib test_name -- --nocapture

# Run tests in a specific crate
cargo test -p crate_name

# Run tests with output
cargo test -- --nocapture

# Run a specific test with backtrace
RUST_BACKTRACE=1 cargo test test_name -- --nocapture
```

### Bacon Commands (Interactive)
```bash
# Press keys in bacon to run jobs:
c   # clippy-all
p   # pedantic (strict checks)
b   # check (default)
```

### Git Hooks (Automated)
Pre-commit runs: format check, clippy pedantic, cargo check
Pre-push runs: cargo test, cargo doc

## Code Style Guidelines

### Imports
- Group imports in 3 sections: std, external crates, internal crates
- Sort within each group alphabetically
- Use `use crate::` for internal module references
- Remove unused imports (clippy enforces this)
- Import specific types rather than wildcards, except for traits

```rust
// Good
use std::time::Duration;

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::actor::Host;
use crate::state::HostState;
```

### Formatting & Naming
- Run `cargo fmt --all` before committing (enforced by lefthook)
- Use `snake_case` for variables, functions, and module names
- Use `PascalCase` for types, structs, enums, traits
- Use `SCREAMING_SNAKE_CASE` for constants
- Maximum line length is enforced by fmt (100 chars)
- Indentation: 4 spaces (no tabs)

### Documentation
- All public items MUST have doc comments (`///`)
- Use backticks around code identifiers: `` `FieldName`, `function()`, `Type` ``
- Clippy strictly enforces doc-markdown, so `OpenAPI` → `` `OpenAPI` ``
- Example in docs:
  ```rust
  /// Returns the host state.
  ///
  /// # Example
  /// ```
  /// let state = host.state();
  /// ```
  pub fn state(&self) -> HostState {
  ```

### Types & Generics
- Always use explicit types in public APIs (no type inference)
- Use `&str` for string slices, `String` for owned strings
- Use `Result<T, E>` for fallible operations (never `Option` for errors)
- Prefer `&[T]` over `Vec<T>` in function parameters when possible
- Use strong typing over stringly-typed code

### Error Handling
- Use `thiserror::Error` for library error types in all `lib` crates
- Use `color_eyre::Result` in binary crates (`src/main.rs`)
- Use `eyre::Result` in library crates where color-eyre isn't available
- Always provide context: `error.context("what failed")?`
- Define custom error enums with the `#[derive(thiserror::Error)]` macro
- Never use `.unwrap()` or `.expect()` in production code (only in examples/tests)

```rust
use thiserror::Error;

#[derive(Error, Debug)]
pub enum HostError {
    #[error("SSH connection failed: {0}")]
    SshFailed(String),
    
    #[error("Package update failed: {0}")]
    UpdateFailed(#[from] std::io::Error),
}
```

### Async/Await
- All I/O operations MUST be async (using `tokio`)
- Traits: use `#[async_trait]` on impl blocks that use async functions
- Spawning tasks: use `tokio::spawn()` for fire-and-forget, store `JoinHandle` if you need to wait
- Always `.await` Future values (clippy enforces this)

### Lifetimes
- Omit lifetime parameters when Rust can infer them (lifetime elision rules)
- Name lifetimes descriptively: `'req`, `'session`, `'actor` (not `'a`, `'b`)
- Self-referential structs require explicit lifetimes (rarely needed in this codebase)

### Module Organization
```
crate/
├── lib.rs                 # Re-exports public APIs
├── actor/                 # Actor implementations
│   ├── mod.rs            # Public exports
│   ├── orchestrator.rs    # OrchestratorActor impl
│   └── host.rs           # HostActor impl
├── message.rs            # Message type definitions
└── state.rs              # State machine types
```

- Keep modules focused and single-responsibility
- Use `pub mod` in `mod.rs` to re-export important items
- Mark internal modules as private by default

### Traits & Generics
- Trait methods should have concrete return types (avoid overly generic impls)
- Use trait bounds where necessary, avoid `impl Trait` in structs (use generics)
- Implement `Debug` and `Clone` for types that benefit from them
- Use `Send + Sync` bounds for thread-safe types in actor messages

### Testing
- Tests live in `#[cfg(test)]` mod blocks at end of files or in `tests/` directory
- Use descriptive test names: `test_host_transitions_from_idle_to_updating()`
- Test happy path and error cases
- Mock external dependencies; use fixture functions for setup

## Crate Structure & Dependencies

### Library Crates (must have `thiserror`)
- tendhost-api: Shared types (requests, responses, events)
- tendhost-core: Actors, messages, state machines
- tendhost-inventory: osquery integration
- tendhost-pkg: Package manager abstraction
- tendhost-exec: Remote execution (SSH, local)
- tendhost-client: HTTP and WebSocket client

### Binary Crates (can use `color-eyre`)
- tendhost: Daemon (axum, actors)
- tendhost-cli: CLI tool (clap)
- tendhost-tui: Terminal UI (ratatui)

### Workspace Dependencies
All crates reference workspace-defined versions in Cargo.toml.
Do NOT add crate-specific versions unless there's a strong reason.

## Pre-Commit Checklist

Before committing code:

1. Run `cargo fmt --all` (formats in place)
2. Run `cargo clippy --all-targets -- -D warnings` (no warnings allowed)
3. Run `cargo test` (all tests pass)
4. Verify doc comments on public items
5. No `.unwrap()` in production code
6. All imports organized and used

## Common Mistakes to Avoid

- ❌ Using `unwrap()` instead of `?` operator
- ❌ Forgetting backticks in doc comments around code
- ❌ Mixing error types (use consistent Result types)
- ❌ Using `String` in public APIs when `&str` suffices
- ❌ Forgetting `#[async_trait]` on async trait impls
- ❌ Unused imports (clippy catches these)
- ❌ Public fields without doc comments
- ❌ Overly generic type parameters that obscure intent

## MCP Tools

When available, use MCP tools for enhanced capabilities:
- `context7`: For fetching up-to-date library documentation
- `grepmcp`: For searching code patterns across GitHub repositories
- `playwright`: For browser automation and testing web interfaces

## Project Philosophy

- **Actor-based**: Use kameo for isolated concurrent components
- **Type-safe**: Strong typing over stringly-typed approaches
- **Explicit**: Prefer explicit over implicit (except where Rust's inferred)
- **Tested**: All public APIs should have tests
- **Documented**: All public items documented with examples
- **Production-ready**: No panics, proper error handling, structured logging
