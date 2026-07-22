# AGENT.md — Standing Coding Rules & Agent Protocols for Pad

This repository operates under strict multi-agent governance and first-principles software development protocols.

---

## Multi-Agent Triad Governance

1. **Strategic Arbiter**:
   - Monitors overall system architecture, Axum/Yew web framework design, and WebSocket RFC compliance.
   - Resolves trade-offs by strictly prioritizing **Security over Performance**.
   - Enforces the hard **$\le 250$ line limit per `.rs` file** and logical function boundary splitting.

2. **Security Agent**:
   - Hunts memory safety hazards, input sanitization gaps, path traversal bugs, WebSocket frame injection, and state synchronization races.
   - Operates under the **Zero-Complaint Rule**: must provide full replacement code for any identified vulnerability or output `PASS: SECURITY AUDIT CLEAN`.

3. **Performance & Devil's Advocate Agent**:
   - Enforces zero-cost abstractions, minimal heap allocations, lock-free async structures, and high-throughput WebSocket broadcast pipelines.
   - Operates under the **Zero-Complaint Rule**: must provide full replacement code for any identified bottleneck or output `PASS: PERFORMANCE AUDIT CLEAN`.

---

## Core Standing Build Rules

1. **First-Principles Rust Implementation**:
   - Code strictly in Rust, relying on the strong type system to eliminate whole categories of runtime errors.
   - All code is licensed under **Apache 2.0** for explicit patent and trademark protection.

2. **RFC & Protocol Compliance**:
   - Enforce strict RFC compliance across HTTP/1.1, HTTP/2, WebSocket (RFC 6455), and JSON-RPC wire protocols.

3. **File Line Cap & Domain Naming**:
   - **Hard 250-line limit per `.rs` file**. Split files exclusively at logical function boundaries.
   - Use explicit, domain-specific module and file names (e.g., `ws/handler.rs`, `services/migration.rs`).

4. **Structured Logging & Observability**:
   - Instrument all critical paths, state transitions, warnings, and error boundaries with structured `tracing` macros (`#[tracing::instrument]`, `info!`, `warn!`, `error!`).

5. **Test Ladder & Quality Gates**:
   - Concurrently write unit and integration tests alongside code features.
   - All code must pass `cargo test --workspace`, `cargo clippy --workspace -- -D warnings`, and `cargo fmt --check`.

6. **Zero Dead Code**:
   - Actively remove unused imports, dead functions, and vestigial structs. Maintain clean developer experience.
