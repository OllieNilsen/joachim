# AGENTS.md

This repository builds **JOACHIM**, a prompt injection detection engine that uses **Pregroup Grammar** to algebraically prove whether directive illocutionary force scopes over agent-domain actions. The core is a pure Rust library (`joachim-core`) implementing type algebra, a Nussinov-style parser, and a scope checker.

This file defines how coding agents (human or AI) must operate in this repo: **types-first**, **tests-first**, **small iterations**, mathematical correctness, and the OpenSpec workflow.

> Single source of truth for the current implementation plan lives in `openspec/`.
> `AGENTS.md` describes *process*; OpenSpec artifacts describe *what to build*.

---

## 0) Algebraic invariants (do not break)

These are non-negotiable and must hold in every iteration:

1. **9 primitive types only**: `dir`, `ag`, `usr`, `role`, `s`, `n`, `conj`, `ass`, `qst`. Modifiers (`hyp`, `meta`, `neg`, `mod`) are NOT primitives — they are functional types (e.g., `dir · dir^l`).
2. **`SimpleType` is `Copy`**: `{ base: TypeId, adjoint: i8 }`. No `Box`, no heap allocation for types.
3. **`TypeExpr` inner Vec is private**: Access via `as_slice()`, `iter()`, `len()`. Construct via `TypeExpr::new()`, `TypeExpr::unit()`, `From<Vec<SimpleType>>`.
4. **Contraction formula never panics**: `can_contract` uses `checked_sub` and returns `false` on overflow. It is a pure predicate.
5. **Adjoint operations panic on i8 overflow**: `left_adj()`/`right_adj()` use `checked_sub(1).expect()`/`checked_add(1).expect()`. Fail-fast on bad supertagger output.
6. **Parser is infallible**: `parse()` always returns a `LinkageGraph`. Invalid input returns an empty graph, not an error.
7. **Planarity**: No two edges in a `LinkageGraph` may cross. This is enforced by the Nussinov recurrence.
8. **Scope requires contraction**: The two-state BFS requires `has_contracted == true` before reporting an `Ag` target. Pure same-chunk adjacency is never an injection.
9. **Voiding is chunk-granular**: A chunk is entirely voided or entirely not voided. Propagation is BFS over chunk indices, not positions.

If a requested change would violate an invariant: **stop**, explain the conflict, and ask for direction.

---

## 1) Working style: OpenSpec + small iterations

### 1.1 OpenSpec workflow
This repo uses the **OpenSpec** skill-based workflow for planning and implementation:

- **Explore** (`openspec-explore`): Think through problems, investigate, compare options. No code.
- **Propose** (`openspec-propose`): Generate a complete change proposal (design, specs, tasks).
- **Apply** (`openspec-apply-change`): Implement tasks from an active change, one at a time.
- **Archive** (`openspec-archive-change`): Finalize a completed change.

The active change and its artifacts are the implementation plan. Always check `openspec status` before starting work.

### 1.2 Micro-iteration contract
Each task from `tasks.md` is one micro-iteration:

- Implement **one** task (or a small coherent group from the same section).
- Run `cargo check`, `cargo test`, `cargo clippy` after each iteration.
- Property tests (`cargo test` with proptest) must pass.
- Do not batch multiple unrelated sections.

### 1.3 Mandatory stop points
Stop and request review before:

- Any change to the contraction formula or `can_contract` semantics.
- Any change to the Nussinov recurrence or backpointer logic.
- Any change to the scope path definition (adjacency constraint, two-state BFS).
- Any change to voiding propagation rules.
- Any change to the `Verdict` enum or what constitutes an injection.
- Any new dependency beyond `smallvec`, `proptest` (dev), `serde`/`serde_json` (optional).

---

## 2) Types-first, tests-first

### 2.1 Types-first
Before implementing behavior, define the types at the boundary. The spec has precise Rust struct definitions — follow them exactly.

**Rules**
- `SimpleType`, `NodeMeta`, `LinkageEdge` are `Copy`. Keep them that way.
- `TypeId` and `VoidingKind` are `Copy` enums. No `String` payloads.
- `Verdict` is `#[non_exhaustive]`. Downstream match arms must have a wildcard.
- Newtypes and strong typing where specified: `u16` for positions and chunk indices, `i8` for adjoint.
- `TypeExpr` inner field is private. No `.0` access.

### 2.2 Tests-first
Add tests before or alongside implementation. The spec defines exact test expectations.

**Minimum test expectations by change type**
- Type algebra changes: property-based tests (section 5 of tasks.md).
- Parser changes: unit tests (section 7) + property tests (section 8).
- Scope checker changes: unit tests (section 9) + property tests (section 10).
- Integration: full parse-then-check pipeline tests from `test_fixtures.rs` (section 11).

**Property test coverage is not optional.** The algebraic laws must hold for all generated inputs. Use `proptest` with `Arbitrary` implementations for `SimpleType`, `TypeExpr`, `VoidingKind`, `TypeAssignment`, and `ParseInput`.

---

## 3) Rust best practices

### 3.1 Safety baseline
- Default: `#![forbid(unsafe_code)]` in `joachim-core`.
- No `unwrap()` in non-test code. Use `expect()` only where the spec explicitly requires panics (adjoint overflow).
- `can_contract` must never panic. Parser must never panic. Scope checker must never panic.

### 3.2 Error handling
- The parser returns `LinkageGraph` (infallible), not `Result`.
- Invalid input → empty graph with `timed_out: false`.
- Timeout → best-effort graph with `timed_out: true`.
- No `Result` types in the public API of `joachim-core` for MVP.

### 3.3 Performance awareness
- `SimpleType` is `Copy` and 2 bytes. Keep it on the stack.
- Nussinov DP table is O(n^2) space per segment. For n <= 250, this is ~62KB — fine.
- Adjacency list uses `SmallVec<[(u16, EdgeKind); 4]>` to avoid heap allocation for typical node degrees.
- Edges sorted by `left` in `LinkageGraph` for binary search.
- Scope checker BFS over chunks (max ~50), not positions (max ~250).

### 3.4 Dependency hygiene
Allowed dependencies:
- `smallvec` — stack-allocated adjacency lists in scope checker.
- `proptest` — dev-dependency for property-based testing.
- `serde` + `serde_json` — optional, behind `serde` feature flag.

Any new dependency must be justified. Prefer zero-dep solutions.

---

## 4) Mathematical correctness

This is a formal algebraic system. Correctness is not negotiable.

### 4.1 Contraction
The contraction formula is:
```rust
fn can_contract(x: SimpleType, y: SimpleType) -> bool {
    x.base == y.base && y.adjoint.checked_sub(1).map_or(false, |r| x.adjoint == r)
}
```
Do not simplify to `x.adjoint == y.adjoint - 1` (i8 overflow). Do not use `wrapping_sub`.

### 4.2 Nussinov recurrence
```
dp[i][j] = max(
    dp[i+1][j],
    max over k in (i+1..=j) where can_contract(seq[i], seq[k]):
        dp[i+1][k-1] + dp[k+1][j] + 1
)
```
DP uses **segment-local** indices. Extraction translates to **global** positions via `+ segment_offset`. All `LinkageEdge` values use global positions.

### 4.3 Scope paths
Alternating steps: adjacent same-chunk (`|i-j| == 1 && same chunk_idx`) and contraction edges. Must include at least one contraction step (two-state BFS with `has_contracted` flag).

### 4.4 Voiding propagation
BFS over **chunk indices** (not positions). Seed with `voiding: Some(_)` chunks. Expand via contraction edges crossing chunk boundaries. A chunk is voided if it's a seed or reachable from a seed.

---

## 5) Crate structure

```
joachim/
├── Cargo.toml                    (workspace root)
├── crates/
│   └── joachim-core/
│       ├── Cargo.toml            (smallvec, proptest dev-dep, serde optional)
│       ├── src/
│       │   ├── lib.rs
│       │   ├── types.rs          (TypeId, SimpleType, TypeExpr, VoidingKind, TypeAssignment)
│       │   ├── linkage.rs        (NodeMeta, LinkageEdge, LinkageGraph)
│       │   ├── parser.rs         (ParseInput, validate, flatten, Nussinov DP, parse)
│       │   └── scope.rs          (ScopePattern, ScopeViolation, Verdict, check_scope)
│       └── tests/
│           ├── type_properties.rs
│           ├── parser_properties.rs
│           ├── scope_properties.rs
│           ├── integration.rs
│           └── test_fixtures.rs  (ground truth from type_annotations_v0.md)
│
├── openspec/                     (spec artifacts — do not edit during implementation)
├── type_annotations_v0.md        (hand-annotated corpus — ground truth)
├── test_corpus_v0.json           (raw test examples)
└── ROADMAP.md
```

---

## 6) Key reference files

| File | Purpose |
|------|---------|
| `openspec/changes/pregroup-core/design.md` | All design decisions (D1-D11), authoritative |
| `openspec/changes/pregroup-core/tasks.md` | Implementation task list, follow in order |
| `openspec/changes/pregroup-core/specs/pregroup-types/spec.md` | Type algebra requirements and scenarios |
| `openspec/changes/pregroup-core/specs/pregroup-parser/spec.md` | Parser requirements (Nussinov, validation, Pass 2) |
| `openspec/changes/pregroup-core/specs/scope-checker/spec.md` | Scope checker requirements (two-state BFS, voiding) |
| `openspec/changes/pregroup-core/specs/pregroup-types/properties.md` | Property-based test requirements |
| `type_annotations_v0.md` | Ground truth type assignments for integration tests |

When in doubt about behavior, consult the spec files in order: `design.md` > `specs/` > `tasks.md`.

---

## 7) Pre-review checklist

Before requesting review on any iteration:

```bash
cargo fmt --all -- --check
cargo clippy --workspace -- -D warnings
cargo test --workspace
cargo test --workspace --features serde   # if serde code was touched
```

All must pass. Property tests are included in `cargo test`.

---

## 8) If anything is unclear: stop and ask

Do not guess:
- Type algebra semantics (consult `design.md` Decisions 1-2)
- Contraction edge cases (consult `types/spec.md`)
- Scope path traversal rules (consult `design.md` Decision 9)
- Voiding propagation boundaries (consult `design.md` Decision 10)
- What constitutes an injection (consult `scope-checker/spec.md`)

Ask for the relevant spec section or guidance first.
