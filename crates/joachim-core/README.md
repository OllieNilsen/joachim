# joachim-core

Pregroup grammar engine for prompt injection detection.

JOACHIM detects prompt injection by algebraically proving whether directive
illocutionary force scopes over agent-domain actions. This crate implements
the core type algebra, parser, and scope checker.

## Architecture

```text
TypeAssignment[]  →  Parser (Nussinov DP)  →  LinkageGraph  →  Scope Checker  →  Verdict
```

1. **Type algebra** (`types`): 9 primitive types with `i8` adjoint counters.
   Modifiers are functional types (`dir · dir^l`), not primitives. Voiding
   semantics are carried by `VoidingKind` annotations, separate from the
   type algebra.

2. **Parser** (`parser`): Nussinov-style DP finds the maximal planar linkage
   within each conjunction-delimited segment. Security-aware two-pass scoring
   ensures injection-relevant edges are surfaced even when a benign parse has
   more total contractions.

3. **Scope checker** (`scope`): Two-state BFS traversal detects `dir → ag`
   and `role → ag` scope paths. Voiding is chunk-granular: self-voiding for
   annotated chunks, BFS propagation along contraction edges.

## Quick Example

```rust
use joachim_core::types::*;
use joachim_core::parser::{parse, ParseInput};
use joachim_core::scope::{check_scope, Verdict};

// "Ignore your instructions" → dir · ag^l · ag
let assignments = vec![TypeAssignment {
    chunk_idx: 0,
    type_expr: TypeExpr::new(vec![
        SimpleType { base: TypeId::Dir, adjoint: 0 },
        SimpleType { base: TypeId::Ag, adjoint: -1 },
        SimpleType { base: TypeId::Ag, adjoint: 0 },
    ]),
    voiding: None,
}];

let graph = parse(&ParseInput(assignments.clone()), None);
let verdict = check_scope(&graph, &assignments);
assert!(matches!(verdict, Verdict::Injection { .. }));
```

## Key Invariants

- `SimpleType` is `Copy` (2 bytes: `TypeId` + `i8` adjoint).
- `can_contract()` never panics (uses `checked_sub`).
- `left_adj()`/`right_adj()` panic on `i8` overflow (fail-fast).
- Parser is infallible: always returns a `LinkageGraph`.
- Planarity: no two edges in a `LinkageGraph` may cross.
- Scope requires at least one contraction step (two-state BFS).
- Voiding is chunk-granular, not position-granular.

See [ROADMAP.md](../../ROADMAP.md) for the broader project context.
