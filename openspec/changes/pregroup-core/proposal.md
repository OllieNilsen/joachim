## Why

JOACHIM's detection approach requires a formal algebraic engine that can prove whether directive illocutionary force scopes over agent-domain actions. This is the deterministic core that makes the system explainable and auditable. Without a working pregroup parser and scope checker, we cannot validate that the type-theoretic approach works at all. This is the foundational component that everything else builds on.

## What Changes

- Add `joachim-core` crate implementing pregroup grammar primitives
- Implement two-level type system: `SimpleType` (atomic with i8 adjoint counter, overflow-safe contraction via `checked_sub`) and `TypeExpr` (product of simple types, private inner Vec)
- Implement `VoidingKind` annotation on `TypeAssignment` to separate algebraic structure from semantic voiding judgments
- Implement two-pass Nussinov parser: Pass 1 finds maximal planar linkage by total contraction count, Pass 2 (triggered only when Pass 1 surfaces no injection) re-runs with injection-aware bonus scoring
- Implement conjunction (`conj`) as an opaque segment barrier
- Implement scope checker with two-state BFS traversal, chunk-granular voiding, and input validation
- Property-based test suite proving algebraic laws hold
- Integration tests using hand-annotated corpus examples via single-source-of-truth test fixtures

## Capabilities

### New Capabilities

- `pregroup-types`: Two-level type algebra. `SimpleType` is a `Copy` struct with `TypeId` base and `i8` adjoint counter (9 primitives: dir, ag, usr, role, s, n, conj, ass, qst). Overflow-safe contraction via `checked_sub`. `left_adj`/`right_adj` panic on i8 bounds (fail-fast). `TypeExpr` has private inner Vec with `as_slice()`/`iter()`/`len()` accessors. Modifiers are functional types (e.g., `dir · dir^l`), not primitives. `VoidingKind` enum (`Hypothetical`, `Negation`, `Meta`) annotates chunks separately from the type algebra. All types support serde behind feature flag.
- `pregroup-parser`: Two-pass Nussinov parser. Pass 1 fills a standard Nussinov DP table (segment-local indices, translated to global on extraction) to find the maximal non-crossing matching within each conj-delimited segment. Pass 2 (conditional) re-runs with injection-aware bonus scoring if Pass 1 surfaced no injection-relevant edges. Conjunction positions act as opaque segment barriers (position-level, supporting multi-element conj chunks). Input validation rejects non-monotonic chunk indices and empty TypeExprs. Produces a `LinkageGraph` with `Vec<NodeMeta>` (node identity is vector index) and sorted `Vec<LinkageEdge>`. Always returns a result. Optional timeout.
- `scope-checker`: Analyzes `LinkageGraph` via two-state BFS traversal (tracking `has_contracted` flag) following positionally-adjacent same-chunk steps and contraction edges. Requires at least one contraction step per scope path. Detects `dir → ag` and `role → ag` scope paths (including intra-chunk self-contraction). Voiding is chunk-granular: (1) self-voiding — voiding-annotated chunks are entirely voided; (2) BFS propagation — contraction edges from voided chunks void target chunks transitively. Produces a `#[non_exhaustive] Verdict` (Injection with position references, or Clean).

### Modified Capabilities

(None — this is a new crate)

## Impact

- **New crate**: `crates/joachim-core/` added to workspace
- **Cargo.toml**: Updated to workspace configuration with joachim-core member
- **Dependencies**: `smallvec` (stack-allocated adjacency lists); `proptest` (dev-dependency); `serde` + `serde_json` (optional, behind `serde` feature flag)
- **Test fixtures**: Single-source-of-truth `test_fixtures.rs` module encoding examples from `type_annotations_v0.md`
