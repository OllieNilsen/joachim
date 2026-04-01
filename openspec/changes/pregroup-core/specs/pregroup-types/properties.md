## ADDED Requirements

### Requirement: Property-based test coverage
The system SHALL include property-based tests using `proptest` that verify the algebraic laws of pregroup grammar hold for all generated inputs.

#### Scenario: Properties verified by random generation
- **WHEN** property-based tests are run with 1000+ generated cases
- **THEN** all pregroup axioms SHALL hold for every generated type and type sequence

---

## SimpleType Adjunction Properties

### Requirement: Left contraction
For any SimpleType `a`: left adjoint followed by base contracts.
`a^l · a → 1`

Using the contraction formula: `can_contract(x, y) = x.base == y.base && y.adjoint.checked_sub(1).map_or(false, |r| x.adjoint == r)`

#### Scenario: Left contraction holds
- **WHEN** given any randomly generated SimpleType `a`
- **THEN** `can_contract(a.left_adj(), a)` SHALL be true

### Requirement: Right contraction
For any SimpleType `a`: base followed by right adjoint contracts.
`a · a^r → 1`

#### Scenario: Right contraction holds
- **WHEN** given any randomly generated SimpleType `a`
- **THEN** `can_contract(a, a.right_adj())` SHALL be true

### Requirement: Mixed adjoint cancellation
Left and right adjoints are mutual inverses: `(a^l)^r = a` and `(a^r)^l = a`.

With i8 representation: decrementing then incrementing (or vice versa) returns to the original adjoint count.

#### Scenario: Left-right cancellation
- **WHEN** given any randomly generated SimpleType `a`
- **THEN** `a.left_adj().right_adj()` SHALL equal `a`

#### Scenario: Right-left cancellation
- **WHEN** given any randomly generated SimpleType `a`
- **THEN** `a.right_adj().left_adj()` SHALL equal `a`

### Requirement: Double adjoint does NOT simplify
`(a^l)^l ≠ a` and `(a^r)^r ≠ a`. The individual adjoint operations are not involutions.

With i8 representation: `adjoint - 2 != adjoint` and `adjoint + 2 != adjoint`.

#### Scenario: Double left adjoint is distinct
- **WHEN** given any randomly generated SimpleType `a`
- **THEN** `a.left_adj().left_adj()` SHALL NOT equal `a`

#### Scenario: Double right adjoint is distinct
- **WHEN** given any randomly generated SimpleType `a`
- **THEN** `a.right_adj().right_adj()` SHALL NOT equal `a`

### Requirement: Contraction only with matching types
Contraction SHALL only succeed when both base AND adjoint relationship match.

#### Scenario: Mismatched base types don't contract
- **WHEN** given distinct TypeIds `A ≠ B` and any adjoint values
- **THEN** `can_contract(SimpleType(A, n), SimpleType(B, m))` SHALL be false for all n, m

#### Scenario: Mismatched adjoint gap doesn't contract
- **WHEN** given any TypeId `A` and adjoint values where `n != m - 1`
- **THEN** `can_contract(SimpleType(A, n), SimpleType(A, m))` SHALL be false

### Requirement: Contraction never panics
`can_contract` SHALL never panic, even for extreme adjoint values.

#### Scenario: i8::MIN adjoint doesn't panic
- **WHEN** given `can_contract(SimpleType(A, -128), SimpleType(A, -128))`
- **THEN** the result SHALL be `false` (no panic)

#### Scenario: i8::MIN right operand doesn't panic
- **WHEN** given `can_contract(SimpleType(A, any), SimpleType(A, -128))`
- **THEN** the result SHALL be `false` (checked_sub prevents underflow)

### Requirement: SimpleType equality is an equivalence relation

#### Scenario: Reflexive
- **WHEN** given any SimpleType `a`
- **THEN** `a == a` SHALL be true

#### Scenario: Symmetric
- **WHEN** given SimpleTypes `a` and `b` where `a == b`
- **THEN** `b == a` SHALL be true

#### Scenario: Transitive
- **WHEN** given SimpleTypes `a`, `b`, `c` where `a == b` and `b == c`
- **THEN** `a == c` SHALL be true

---

## TypeExpr Monoid Properties

### Requirement: TypeExpr concatenation associativity
For any TypeExprs `a`, `b`, `c`: concatenation SHALL be associative.
`(a · b) · c = a · (b · c)`

(Where `TypeExpr` concatenation means concatenating their `Vec<SimpleType>` contents.)

#### Scenario: Associativity holds
- **WHEN** given any three randomly generated TypeExprs `a`, `b`, `c`
- **THEN** `concat(concat(a, b), c)` SHALL equal `concat(a, concat(b, c))`

### Requirement: TypeExpr unit is identity
The empty TypeExpr (unit) SHALL be both left and right identity for concatenation.
`1 · a = a` and `a · 1 = a`

#### Scenario: Left identity holds
- **WHEN** given any randomly generated TypeExpr `a`
- **THEN** `concat(TypeExpr::unit(), a)` SHALL equal `a`

#### Scenario: Right identity holds
- **WHEN** given any randomly generated TypeExpr `a`
- **THEN** `concat(a, TypeExpr::unit())` SHALL equal `a`

---

## TypeExpr Adjoint Properties

### Requirement: TypeExpr left adjoint reverses and adjoins
The left adjoint of a TypeExpr reverses the sequence and takes the left adjoint of each element.
`(a1 · a2 · ... · an)^l = an^l · ... · a2^l · a1^l`

With i8 representation: reverse the vec and decrement each adjoint by 1.

#### Scenario: Left adjoint is contravariant
- **WHEN** given any randomly generated TypeExpr with elements `[a1, a2, ..., an]`
- **THEN** `left_adj(expr)` SHALL equal `TypeExpr([an^l, ..., a2^l, a1^l])`

### Requirement: TypeExpr right adjoint reverses and adjoins
The right adjoint of a TypeExpr reverses the sequence and takes the right adjoint of each element.
`(a1 · a2 · ... · an)^r = an^r · ... · a2^r · a1^r`

With i8 representation: reverse the vec and increment each adjoint by 1.

#### Scenario: Right adjoint is contravariant
- **WHEN** given any randomly generated TypeExpr with elements `[a1, a2, ..., an]`
- **THEN** `right_adj(expr)` SHALL equal `TypeExpr([an^r, ..., a2^r, a1^r])`

### Requirement: TypeExpr adjoint involution
Mixed adjoint application on TypeExpr cancels: `(e^l)^r = e` and `(e^r)^l = e`.

#### Scenario: Left-right cancellation on TypeExpr
- **WHEN** given any randomly generated TypeExpr `e`
- **THEN** `right_adj(left_adj(e))` SHALL equal `e`

#### Scenario: Right-left cancellation on TypeExpr
- **WHEN** given any randomly generated TypeExpr `e`
- **THEN** `left_adj(right_adj(e))` SHALL equal `e`

---

## Parser Properties

### Requirement: Parse determinism
Given the same input, the parser SHALL produce the same `LinkageGraph`.

#### Scenario: Deterministic parsing
- **WHEN** parsing the same type assignment sequence twice
- **THEN** both parse attempts SHALL return identical `LinkageGraph`s (same meta, same edges)

### Requirement: Reduction termination
All parse invocations SHALL terminate in finite steps.

#### Scenario: No infinite loops
- **WHEN** parsing any randomly generated type sequence of length <= 20
- **THEN** the parser SHALL terminate within bounded time

### Requirement: Edge validity
Every edge in the `LinkageGraph` SHALL represent a valid contraction.

#### Scenario: All edges are valid contractions
- **WHEN** parsing produces a `LinkageGraph` with edges
- **THEN** for each edge `(left, right)`, `can_contract(meta[left].simple_type, meta[right].simple_type)` SHALL be true

### Requirement: Planarity
No two edges in the `LinkageGraph` SHALL cross.

#### Scenario: No crossing edges
- **WHEN** parsing produces edges `(a, b)` and `(c, d)` with `a < c`
- **THEN** either `b < c` (disjoint) or `a < c < d < b` (nested) — never `a < c < b < d` (crossing)

### Requirement: Edge count bound
The number of edges SHALL be bounded by the input size.

#### Scenario: At most n/2 edges
- **WHEN** parsing a flattened sequence of n simple types
- **THEN** the graph SHALL contain at most floor(n/2) edges

### Requirement: Known-contractable sequences always contract
A sequence constructed to be contractable SHALL always produce at least one edge.

#### Scenario: Adjoint pair contracts
- **WHEN** parsing the sequence `[a^l, a]` for any SimpleType `a` (with `a.adjoint == 0`)
- **THEN** the graph SHALL contain exactly 1 edge

### Requirement: Nested pairs produce maximal linkage
A sequence with nested contractable pairs SHALL produce all possible non-crossing edges.

#### Scenario: Nested pairs
- **WHEN** parsing `[a^l, b^l, b, a]`
- **THEN** the graph SHALL contain 2 edges: `(0, 3)` and `(1, 2)`

### Requirement: Conjunction barrier respected
No edge SHALL cross a `conj` position.

#### Scenario: No edges across conjunction
- **WHEN** parsing a sequence containing `conj` at position k
- **THEN** no edge `(i, j)` SHALL have `i < k < j`

---

## Scope Checker Properties

### Requirement: Verdict determinism
The same `LinkageGraph` and `TypeAssignment`s SHALL always produce the same verdict.

#### Scenario: Deterministic verdicts
- **WHEN** checking the same graph and assignments twice
- **THEN** both checks SHALL return identical verdicts

### Requirement: Scope paths use positional adjacency
Same-chunk steps in scope paths SHALL only move between positionally adjacent positions (`|i - j| == 1`).

#### Scenario: Adjacent step valid
- **WHEN** positions 0 and 1 share a chunk
- **THEN** stepping 0→1 SHALL be allowed in scope path traversal

#### Scenario: Non-adjacent same-chunk step rejected
- **WHEN** positions 0 and 2 share a chunk but position 1 is in a different chunk
- **THEN** stepping 0→2 SHALL NOT be allowed (they are not positionally adjacent)

### Requirement: Scope path requires contraction step
A path consisting entirely of same-chunk adjacency steps (no contraction edges) SHALL NOT be reported as a scope relationship.

#### Scenario: Intra-chunk dir · ag with no edges is not flagged
- **WHEN** a single chunk has TypeExpr `dir · ag` and the graph has 0 contraction edges
- **THEN** the verdict SHALL be Clean (the two-state BFS never sets `has_contracted = true`)

### Requirement: Self-voiding (chunk-granular)
All injection patterns within a voiding-annotated chunk are voided.

#### Scenario: Meta chunk self-voids
- **WHEN** a chunk has `voiding: Some(Meta)` and contains an intra-chunk `dir → ag` path
- **THEN** the verdict SHALL be Clean

### Requirement: Voiding transitivity via chunk-level propagation
If a voiding chunk has a contraction edge to another chunk, that target chunk becomes voided. Propagation continues transitively.

#### Scenario: Nested voiding
- **WHEN** voided chunk 0 has a contraction edge to chunk 1, and chunk 1 has a contraction edge to chunk 2
- **THEN** chunks 1 AND 2 SHALL be voided

### Requirement: Dir-over-usr never flagged
Directives scoping over user-domain types SHALL never produce an Injection verdict.

#### Scenario: Dir-over-usr is clean
- **WHEN** a graph contains only `dir → usr` paths and no `dir → ag` or `role → ag`
- **THEN** the verdict SHALL be Clean

### Requirement: Unvoided dir-over-ag always flagged
Any unvoided `dir → ag` path SHALL produce an Injection verdict.

#### Scenario: No voiding means injection
- **WHEN** a graph contains a `dir → ag` path and no voiding annotations on the path's chunks
- **THEN** the verdict SHALL be Injection

### Requirement: Non-voiding modifier does not void
A chunk with `voiding: None` SHALL NOT void its linked target, regardless of its type structure.

#### Scenario: Structurally identical but non-voiding
- **WHEN** a chunk typed `dir · dir^l` with `voiding: None` links to a `dir → ag` path
- **THEN** the verdict SHALL be Injection (not voided)

### Requirement: Empty graph is clean
A graph with no edges SHALL produce a Clean verdict.

#### Scenario: No edges means clean
- **WHEN** a graph contains only unlinked nodes
- **THEN** the verdict SHALL be Clean
