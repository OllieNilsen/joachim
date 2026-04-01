## 1. Project Setup

- [x] 1.1 Convert root Cargo.toml to workspace configuration with `crates/` directory; move existing `src/main.rs` out or remove placeholder
- [x] 1.2 Create `crates/joachim-core/Cargo.toml` with `proptest` dev-dependency and optional `serde` feature flag
- [x] 1.3 Set up module structure: `lib.rs`, `types.rs`, `parser.rs`, `scope.rs`, `linkage.rs`

## 2. SimpleType (atomic types with i8 adjoints)

- [x] 2.1 Implement `TypeId` enum with 9 primitive types: `Dir`, `Ag`, `Usr`, `Role`, `S`, `N`, `Conj`, `Ass`, `Qst`; derive `Copy, Clone, PartialEq, Eq, Hash, Debug`
- [x] 2.2 Implement `SimpleType { base: TypeId, adjoint: i8 }` with `Copy, Clone, PartialEq, Eq, Hash, Debug` derives
- [x] 2.3 Implement `SimpleType::new(base: TypeId) -> Self` constructor (adjoint=0)
- [x] 2.4 Implement `SimpleType::left_adj(self) -> Self`: `self.adjoint.checked_sub(1).expect("adjoint underflow")`
- [x] 2.5 Implement `SimpleType::right_adj(self) -> Self`: `self.adjoint.checked_add(1).expect("adjoint overflow")`
- [x] 2.6 Implement `SimpleType::base(self) -> TypeId` (returns the base, ignoring adjoint)
- [x] 2.7 Implement `can_contract(left: SimpleType, right: SimpleType) -> bool`: `left.base == right.base && right.adjoint.checked_sub(1).map_or(false, |r| left.adjoint == r)` — pure predicate, never panics
- [x] 2.8 Implement `Display` for `TypeId`: lowercase name (`dir`, `ag`, etc.)
- [x] 2.9 Implement `Display` for `SimpleType`: base name + `^l` repeated for negative adjoint, `^r` repeated for positive adjoint
- [x] 2.10 Implement `Arbitrary` for `SimpleType` (proptest): generate random TypeId with adjoint in range [-3, 3]
- [x] 2.11 Implement `Serialize`/`Deserialize` for `TypeId` and `SimpleType` behind `serde` feature flag

## 3. TypeExpr (product of simple types)

- [x] 3.1 Implement `TypeExpr(Vec<SimpleType>)` with **private** inner field; derive `Clone, PartialEq, Eq, Debug`
- [x] 3.2 Implement `TypeExpr::new(types: Vec<SimpleType>) -> Self` constructor
- [x] 3.3 Implement `TypeExpr::unit() -> Self` returning empty vec
- [x] 3.4 Implement `TypeExpr::concat(self, other: TypeExpr) -> TypeExpr` consuming both inputs via `Vec::extend`
- [x] 3.5 Implement `TypeExpr::left_adj(&self) -> TypeExpr`: reverse elements, decrement each adjoint by 1
- [x] 3.6 Implement `TypeExpr::right_adj(&self) -> TypeExpr`: reverse elements, increment each adjoint by 1
- [x] 3.7 Implement `TypeExpr::as_slice(&self) -> &[SimpleType]`, `is_unit()`, `len()`, `iter()`
- [x] 3.8 Implement `From<Vec<SimpleType>> for TypeExpr`
- [x] 3.9 Implement `Display` for `TypeExpr`: `·`-separated simple types, or `1` for empty
- [x] 3.10 Implement `Arbitrary` for `TypeExpr` (proptest): generate random non-empty exprs of bounded length (1..=5)
- [x] 3.11 Implement `Serialize`/`Deserialize` for `TypeExpr` behind `serde` feature flag

## 4. VoidingKind and TypeAssignment

- [x] 4.1 Implement `VoidingKind` enum: `Hypothetical`, `Negation`, `Meta`; derive `Copy, Clone, PartialEq, Eq, Debug`
- [x] 4.2 Implement `TypeAssignment { chunk_idx: u16, type_expr: TypeExpr, voiding: Option<VoidingKind> }`
- [x] 4.3 Implement `Display` for `TypeAssignment` showing chunk index, type, and voiding annotation
- [x] 4.4 Implement `Serialize`/`Deserialize` for `VoidingKind` and `TypeAssignment` behind `serde` feature flag
- [x] 4.5 Implement `Arbitrary` for `VoidingKind` (proptest): uniform over 3 variants
- [x] 4.6 Implement `Arbitrary` for `TypeAssignment` (proptest): random TypeExpr (non-empty), random `Option<VoidingKind>`, chunk_idx placeholder (set by ParseInput generator)

## 5. Type System Property Tests

- [x] 5.1 Property: Left contraction — `can_contract(a.left_adj(), a)` is true for all SimpleTypes
- [x] 5.2 Property: Right contraction — `can_contract(a, a.right_adj())` is true for all SimpleTypes
- [x] 5.3 Property: Contraction formula consistency — `can_contract(x, y)` iff `x.base == y.base && y.adjoint.checked_sub(1).map_or(false, |r| x.adjoint == r)`
- [x] 5.4 Property: Mixed adjoint cancellation — `a.left_adj().right_adj() == a` and `a.right_adj().left_adj() == a`
- [x] 5.5 Property: Double left adjoint is distinct — `a.left_adj().left_adj() != a`
- [x] 5.6 Property: Double right adjoint is distinct — `a.right_adj().right_adj() != a`
- [x] 5.7 Property: Mismatched base types don't contract
- [x] 5.8 Property: Mismatched adjoint gap doesn't contract
- [x] 5.9 Property: TypeExpr concat associativity
- [x] 5.10 Property: TypeExpr unit identity (left and right)
- [x] 5.11 Property: TypeExpr left adjoint is contravariant (reverse + decrement)
- [x] 5.12 Property: TypeExpr right adjoint is contravariant (reverse + increment)
- [x] 5.13 Property: TypeExpr adjoint involution — `right_adj(left_adj(e)) == e` and vice versa
- [x] 5.14 Property: SimpleType equality is reflexive, symmetric, transitive
- [x] 5.15 Property: left_adj panics on i8::MIN adjoint; right_adj panics on i8::MAX adjoint
- [x] 5.16 Property: can_contract never panics — test with all i8 values including i8::MIN, i8::MAX

## 6. LinkageGraph

- [x] 6.1 Implement `NodeMeta { chunk_idx: u16, simple_type: SimpleType }` with `Copy, Clone, Debug` derives
- [x] 6.2 Implement `LinkageEdge { left: u16, right: u16 }` with `Copy, Clone, Debug, PartialEq, Eq` derives
- [x] 6.3 Implement `LinkageGraph { meta: Vec<NodeMeta>, edges: Vec<LinkageEdge>, timed_out: bool }`
- [x] 6.4 Implement `LinkageGraph::edges_from(&self, pos: u16) -> impl Iterator<Item = &LinkageEdge>`: find edges where `left == pos` via binary search; note: finding edges where `right == pos` requires O(E) scan
- [x] 6.5 Implement `LinkageGraph::verify(&self) -> bool`: check all edges are valid contractions, non-crossing, and within bounds
- [x] 6.6 Implement `LinkageGraph::edge_count(&self) -> usize`
- [x] 6.7 Implement `Display` for `LinkageGraph`: human-readable visualization showing positions and edges
- [x] 6.8 Implement `Serialize` for `NodeMeta`, `LinkageEdge`, `LinkageGraph` behind `serde` feature flag

## 7. Parser

- [ ] 7.1 Define `ParseInput(pub Vec<TypeAssignment>)`
- [ ] 7.2 Implement `validate(input: &ParseInput) -> bool`: check chunk_idx monotonically non-decreasing, no empty TypeExprs
- [ ] 7.3 Implement `flatten(input: &ParseInput) -> (Vec<SimpleType>, Vec<u16>)`: expand into flat SimpleType sequence with parallel chunk index vec
- [ ] 7.4 Implement conjunction barrier detection: scan flattened sequence for positions where `base == TypeId::Conj`; return segment boundaries as `Vec<(usize, usize)>` excluding conj positions. Non-conj positions from multi-element conj chunks join the adjacent segment.
- [ ] 7.5 Implement Nussinov DP (Pass 1): for each segment, fill segment-local `dp[i][j]` table using recurrence; store backpointers
- [ ] 7.6 Implement linkage extraction: walk backpointers from `dp[0][seg_len-1]` for each segment; translate segment-local indices to global positions by adding segment offset; produce `Vec<LinkageEdge>`
- [ ] 7.7 Implement injection-relevance check: given extracted edges and chunk index map, determine if any edge is injection-relevant (one endpoint base is `Ag`, other shares chunk with `Dir`/`Role`)
- [ ] 7.8 Implement Nussinov DP (Pass 2): same recurrence but with bonus scoring `+n` for injection-relevant edges; only runs if Pass 1 found zero injection-relevant edges
- [ ] 7.9 Implement `parse(input: &ParseInput, timeout: Option<Duration>) -> LinkageGraph`: validate → flatten → segments → Pass 1 → check → optional Pass 2 → assemble LinkageGraph. Return empty graph on validation failure.
- [ ] 7.10 Add timeout checking: periodically check elapsed time during DP inner loop; set `timed_out` flag on early exit
- [ ] 7.11 Implement `Arbitrary` for `ParseInput` (proptest): generate 1..=10 TypeAssignments with monotonically increasing chunk_idx (0, 1, 2, ...), non-empty random TypeExprs, random voiding
- [ ] 7.12 Write unit test: `[ag^l, ag]` produces 1 edge `(0, 1)`
- [ ] 7.13 Write unit test: `[dir, ag^l, ag]` — edge `(1, 2)`, position 0 unlinked
- [ ] 7.14 Write unit test: `[dir, usr, n]` — 0 edges
- [ ] 7.15 Write unit test: `[a^l, b^l, b, a]` — nested planar edges `(0, 3)` and `(1, 2)`
- [ ] 7.16 Write unit test: empty input returns empty graph
- [ ] 7.17 Write unit test: conjunction barrier — `[ag^l, conj, ag]` produces 0 edges
- [ ] 7.18 Write unit test: conjunction segments — `[ag^l, ag, conj, usr^l, usr]` produces edges `(0,1)` and `(3,4)`
- [ ] 7.19 Write unit test: intra-chunk self-contraction — `[dir, ag^l, ag]` from single chunk produces edge `(1,2)`
- [ ] 7.20 Write unit test: security Pass 2 — ambiguous parse where max-contraction linkage hides injection but Pass 2 surfaces it → Injection
- [ ] 7.21 Write unit test: Pass 2 fallback — when no injection-relevant edges exist in any linkage, Pass 2 returns same result as Pass 1 (identical edge count and edges)
- [ ] 7.22 Write unit test: multi-element conj chunk — `[ag^l, ag, conj, n^r, usr^l, usr]` where `conj · n^r` is one chunk; position 2 excluded, positions 3-5 form segment 2
- [ ] 7.23 Write unit test: input validation — non-monotonic chunk_idx returns empty graph
- [ ] 7.24 Write unit test: input validation — empty TypeExpr returns empty graph

## 8. Parser Property Tests

- [ ] 8.1 Property: Parse determinism — same input always produces same LinkageGraph
- [ ] 8.2 Property: Termination — parsing any randomly generated ParseInput terminates
- [ ] 8.3 Property: Edge validity — `can_contract(meta[e.left].simple_type, meta[e.right].simple_type)` for all edges
- [ ] 8.4 Property: Planarity — no two edges cross
- [ ] 8.5 Property: Edge count bound — edges.len() <= floor(n / 2)
- [ ] 8.6 Property: Adjoint pairs always contract — `[a^l, a]` always produces exactly 1 edge
- [ ] 8.7 Property: No contractions for identical primitives — `[a, a, a]` for any primitive `a` produces 0 edges
- [ ] 8.8 Property: Conjunction barrier — no edge crosses a `conj` position
- [ ] 8.9 Property: Nested pairs — `[a^l, b^l, b, a]` always produces 2 edges
- [ ] 8.10 Property: All edges use global positions — for each edge, `left < meta.len()` and `right < meta.len()`

## 9. Scope Checker

- [ ] 9.1 Define `ScopePattern` enum: `DirOverAg`, `RoleOverAg`; derive `Copy, Clone, PartialEq, Eq, Debug`
- [ ] 9.2 Define `ScopeViolation { pattern: ScopePattern, source_pos: u16, target_pos: u16 }` with vector index references
- [ ] 9.3 Define `#[non_exhaustive] Verdict` enum: `Injection { violations: Vec<ScopeViolation> }`, `Clean`
- [ ] 9.4 Define `EdgeKind` enum: `Contraction`, `Adjacency` — used to tag adjacency list entries
- [ ] 9.5 Implement `build_adjacency(graph: &LinkageGraph) -> Vec<SmallVec<[(u16, EdgeKind); 4]>>`: build bidirectional adjacency list from contraction edges (tagged `Contraction`) + positionally-adjacent same-chunk pairs (tagged `Adjacency`): `|i - j| == 1 && meta[i].chunk_idx == meta[j].chunk_idx`
- [ ] 9.6 Implement `compute_voided_chunks(graph: &LinkageGraph, assignments: &[TypeAssignment]) -> HashSet<u16>`: BFS over chunks — seed with voiding-annotated chunks, expand via contraction edges to non-voided chunks, repeat until fixpoint
- [ ] 9.7 Implement `find_scope_paths(graph: &LinkageGraph, adjacency: &[SmallVec<[(u16, EdgeKind); 4]>]) -> Vec<(u16, u16, ScopePattern)>`: two-state BFS from each `Dir`/`Role` position with state `(pos, has_contracted: bool)`; traverse adjacency edges flipping `has_contracted = true` on `Contraction` edges; report `Ag` positions only when `has_contracted == true`
- [ ] 9.8 Implement `check_scope(graph: &LinkageGraph, assignments: &[TypeAssignment]) -> Verdict`: compute voided chunks, find scope paths, filter out paths where source or target position belongs to a voided chunk, return Injection or Clean
- [ ] 9.9 Handle edge cases: empty graph → Clean, zero-edge graph → Clean

## 10. Scope Checker Property Tests

- [ ] 10.1 Property: Verdict determinism — same graph + assignments always produces same verdict
- [ ] 10.2 Property: Voiding transitivity — construct graph where voiding chunk links to dir→ag, verify Clean
- [ ] 10.3 Property: Self-voiding — construct graph where meta chunk contains intra-chunk dir→ag, verify Clean
- [ ] 10.4 Property: Dir-over-usr is always Clean
- [ ] 10.5 Property: Unvoided dir-over-ag is always Injection
- [ ] 10.6 Property: Non-voiding modifier does NOT void — construct graph with `dir · dir^l` (voiding=None) linking to dir→ag, verify Injection
- [ ] 10.7 Property: Empty graph is Clean
- [ ] 10.8 Property: Positional adjacency — scope path only uses `|i-j|==1` same-chunk steps
- [ ] 10.9 Property: Contraction step required — `dir · ag` in a single chunk with no contraction edges produces Clean (two-state BFS never sets has_contracted)

## 11. Integration Tests

- [ ] 11.1 Create `test_fixtures.rs` module: encode all type annotations from `type_annotations_v0.md` as Rust constants (`Vec<TypeAssignment>` + expected `Verdict` for each example). Single source of truth.
- [ ] 11.2 Create test helper: `fn assert_verdict(assignments: &[TypeAssignment], expected: Verdict)` — parse and check, assert verdict matches
- [ ] 11.3 Test inj_001 (direct override): `[(0, dir · ag^l · ag, None)]` → Injection
- [ ] 11.4 Test inj_002 (stop + ignore): `[(0, dir, None), (1, dir · ag^l · ag, None), (2, dir · usr^l · usr, None)]` → Injection
- [ ] 11.5 Test inj_004 (role hijack): `[(0, role · ag^l · ag, None), (1, qst · usr^l, None)]` → Injection
- [ ] 11.6 Test inj_013 (threat): `[(0, dir · ag^l · ag, None), (1, conj, None), (2, ass, None)]` → Injection
- [ ] 11.7 Test ben_007 (meta discussion): `[(0, n · n^l, Some(Meta)), (1, n · qst · usr^l, None)]` → Clean
- [ ] 11.8 Test ben_009 (hypothetical): `[(0, s · s^l, Some(Hypothetical)), (1, s · dir · ag^l · ag, None), (2, qst · usr^l, None)]` → Clean
- [ ] 11.9 Test ben_011 (quoted content): `[(0, ass, None), (1, n^r · n, None), (2, n · dir · ag^l · ag, Some(Meta)), (3, n^r · n, None)]` → Clean
- [ ] 11.10 Test ben_015 (user-domain directive): `[(0, dir · usr^l, None), (1, usr, None)]` → Clean
- [ ] 11.11 Test ben_018 (negated directive): `[(0, dir · dir^l, Some(Negation)), (1, dir · ag^l · ag, None), (2, dir · usr^l, None), (3, usr, None)]` → Clean
- [ ] 11.12 Test security: ambiguous parse where Pass 1 hides injection, Pass 2 surfaces it → Injection
- [ ] 11.13 Test all fixtures from `test_fixtures.rs` via `#[test_case]` or loop

## 12. Documentation

- [ ] 12.1 Add rustdoc comments to all public types and functions
- [ ] 12.2 Add module-level docs: explain pregroup grammar, i8 adjoint representation, contraction formula (with checked_sub), Nussinov algorithm
- [ ] 12.3 Document VoidingKind and the separation of algebraic structure from semantic voiding
- [ ] 12.4 Document conjunction barrier behavior including multi-element conj chunk handling
- [ ] 12.5 Document intra-chunk self-contraction convention
- [ ] 12.6 Document voiding propagation: self-voiding + chunk-granular BFS
- [ ] 12.7 Document scope path adjacency constraint and two-state BFS
- [ ] 12.8 Document i8 overflow panic behavior (left_adj/right_adj) vs never-panic (can_contract)
- [ ] 12.9 Document input validation rules and behavior on invalid input
- [ ] 12.10 Add doc examples showing: construct types, parse a sequence, check scope
- [ ] 12.11 Add crate-level README with overview, examples, and link to ROADMAP.md
