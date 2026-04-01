## ADDED Requirements

### Requirement: Precise scope-over definition via linkage paths
The system SHALL use the following definition of "scopes over": type `X` scopes over type `Y` if there is a connected path in the linkage graph from a position with base `X` to a position with base `Y`, following steps where each step is one of:

1. **Adjacent same-chunk step**: Move from position `i` to position `j` where `|i - j| == 1` AND `meta[i].chunk_idx == meta[j].chunk_idx`. This restricts same-chunk movement to positionally adjacent neighbours within the TypeExpr product.
2. **Contraction step**: Follow a `LinkageEdge` from one position to the position it contracted with.

A path must consist of at least one contraction step.

**Implementation**: The scope checker SHALL use a **two-state BFS** where the state is `(position: u16, has_contracted: bool)`. The adjacency list encodes both contraction edges (tagged as contraction) and adjacent same-chunk pairs (tagged as adjacency). When traversing a contraction edge, `has_contracted` becomes `true`. An `Ag` position is only reported as a scope target when `has_contracted == true`. This prevents pure same-chunk adjacency walks (e.g., `dir · ag` in a chunk with no contraction edges) from being flagged.

#### Scenario: Directive scopes over agent directly
- **WHEN** chunk 0 has TypeExpr `dir · ag^l` (positions 0, 1) and chunk 1 has `ag` (position 2)
- **AND** the linkage graph contains edge `(1, 2)` connecting `ag^l` to `ag`
- **THEN** the scope checker SHALL determine that `dir` (position 0) scopes over `ag` (position 2): adjacent step 0→1, contraction 1→2.

#### Scenario: Intra-chunk self-contraction scopes
- **WHEN** chunk 0 has TypeExpr `dir · ag^l · ag` (positions 0, 1, 2)
- **AND** the linkage graph contains edge `(1, 2)` from intra-chunk contraction
- **THEN** `dir` (position 0) scopes over `ag` (position 2): adjacent step 0→1, contraction 1→2.

#### Scenario: Transitive scoping via functional modifier
- **WHEN** chunk 0 has `dir · dir^l` (positions 0, 1), chunk 1 has `dir · ag^l` (positions 2, 3), chunk 2 has `ag` (position 4)
- **AND** edges: `(1, 2)` and `(3, 4)`
- **THEN** `dir` (pos 0) scopes over `ag` (pos 4): adjacent 0→1, contraction 1→2, adjacent 2→3, contraction 3→4.

#### Scenario: Unrelated types in same chunk don't create false scope
- **WHEN** chunk 0 has `dir · n` (positions 0, 1) and chunk 1 has `n^r · ag` (positions 2, 3)
- **AND** edge `(1, 2)` connects `n` to `n^r`
- **THEN** the path `dir(0) → n(1) → n^r(2) → ag(3)` IS valid: adjacent 0→1, contraction 1→2, adjacent 2→3. This is a genuine scope path because `dir` produced `n` which flowed into the modifier chain reaching `ag`.

### Requirement: Directive over agent detection
The system SHALL flag linkage paths where `dir` scopes over `ag` as injection pattern `DirOverAg`.

#### Scenario: Direct directive-agent linkage
- **WHEN** `dir · ag^l` connects to `ag` via contraction
- **THEN** the scope checker SHALL flag this as `DirOverAg`

### Requirement: Role over agent detection
The system SHALL flag linkage paths where `role` scopes over `ag` as injection pattern `RoleOverAg`.

#### Scenario: Role-agent linkage
- **WHEN** `role · ag^l` connects to `ag` via contraction
- **THEN** the scope checker SHALL flag this as `RoleOverAg`

### Requirement: Chunk-granular voiding
Voiding operates at **chunk granularity**. A chunk is either entirely voided or entirely not voided. A chunk is voided if:

1. **Self-voiding**: The chunk has `voiding: Some(_)` in its `TypeAssignment`.
2. **Propagated voiding**: Any position in the chunk is the target of a contraction edge originating from a position in an already-voided chunk.

The system SHALL compute the voided chunk set via BFS:
1. Seed: all chunks with `voiding: Some(_)`.
2. Expand: for each voided chunk, find contraction edges to non-voided chunks; add target chunks to voided set.
3. Repeat until fixpoint.

An injection path is voided if the `dir`/`role` source position OR the `ag` target position belongs to a voided chunk.

#### Scenario: Meta chunk's own content is voided
- **WHEN** chunk 0 has TypeExpr `n · dir · ag^l · ag` with `voiding: Some(Meta)`
- **THEN** chunk 0 is voided. The intra-chunk `dir → ag` pattern SHALL NOT be flagged.

#### Scenario: Negation voids target chunk
- **WHEN** chunk 0 is typed `dir · dir^l` with `voiding: Some(Negation)`, and `dir^l` (chunk 0) contracts with `dir` (chunk 1)
- **AND** chunk 1 has `dir · ag^l · ag`
- **THEN** chunk 1 is voided (propagation). The `dir → ag` path from chunk 1 SHALL NOT be flagged.

#### Scenario: Hypothetical voids via sentence-level link
- **WHEN** chunk 0 is typed `s · s^l` with `voiding: Some(Hypothetical)`, and `s^l` contracts with `s` in chunk 1
- **AND** chunk 1 has `s · dir · ag^l · ag`
- **THEN** chunk 1 is voided. The `dir → ag` path SHALL NOT be flagged.

#### Scenario: Non-voiding modifier does NOT void
- **WHEN** chunk 0 is typed `dir · dir^l` with `voiding: None` (e.g., "please")
- **AND** `dir^l` contracts with `dir` in chunk 1
- **THEN** chunk 1 is NOT voided. Any `dir → ag` paths from chunk 1 SHALL be flagged.

#### Scenario: Voiding does not propagate to unlinked chunks
- **WHEN** a voiding chunk has no contraction edges to other chunks
- **THEN** only the voiding chunk itself is voided. Unlinked `dir → ag` patterns elsewhere SHALL be flagged.

#### Scenario: Transitive voiding across multiple chunks
- **WHEN** chunk 0 (voiding) contracts with chunk 1, and chunk 1 contracts with chunk 2
- **THEN** chunk 2 is also voided (BFS propagation reaches it).

### Requirement: Verdict generation
The system SHALL produce a `Verdict` enum marked `#[non_exhaustive]`:
- `Injection { violations: Vec<ScopeViolation> }` if any unvoided `DirOverAg` or `RoleOverAg` path exists.
- `Clean` otherwise.

The `#[non_exhaustive]` annotation ensures future verdict variants (e.g., `Suspicious`, `InsufficientData`) can be added without a semver-breaking change.

#### Scenario: Injection verdict
- **WHEN** analyzing a graph with an unvoided `dir → ag` path
- **THEN** the verdict SHALL be `Injection` with the path referenced

#### Scenario: Clean verdict with no edges
- **WHEN** analyzing a graph with 0 edges
- **THEN** the verdict SHALL be `Clean`

#### Scenario: Clean verdict with voided pattern
- **WHEN** all `dir → ag` paths are voided
- **THEN** the verdict SHALL be `Clean`

#### Scenario: Clean verdict with user-domain only
- **WHEN** the graph contains `dir → usr` paths but no unvoided `dir → ag` or `role → ag`
- **THEN** the verdict SHALL be `Clean`

### Requirement: Multiple pattern detection
The system SHALL detect ALL unvoided injection paths, not just the first.

#### Scenario: Multiple injection patterns
- **WHEN** the graph contains two disconnected unvoided `dir → ag` paths
- **THEN** the verdict SHALL list both

### Requirement: Path reference in verdict
Each injection pattern in the verdict SHALL reference the positions (vector indices) of the start (`dir` or `role`) and end (`ag`) of the path.

#### Scenario: Traceable verdict
- **WHEN** an Injection verdict is produced
- **THEN** each pattern SHALL include the positions of the `dir`/`role` and `ag` nodes

### Requirement: Error handling
The scope checker SHALL handle empty or edge-free graphs gracefully.

#### Scenario: Empty graph
- **WHEN** given a `LinkageGraph` with 0 meta entries and 0 edges
- **THEN** the verdict SHALL be `Clean`

#### Scenario: Graph with only unlinked nodes
- **WHEN** given a `LinkageGraph` with meta entries but 0 edges
- **THEN** the verdict SHALL be `Clean`
