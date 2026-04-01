## ADDED Requirements

### Requirement: TypeAssignment input with voiding annotations
The system SHALL accept a sequence of `TypeAssignment` values, where each contains:
- `chunk_idx: u16` — the chunk's position in the original sequence
- `type_expr: TypeExpr` — the chunk's type assignment
- `voiding: Option<VoidingKind>` — whether this chunk is a voiding operator (and what kind)

#### Scenario: Non-voiding assignment
- **WHEN** the supertagger assigns `dir · ag^l` to chunk 0 with no voiding
- **THEN** the TypeAssignment SHALL have `voiding: None`

#### Scenario: Voiding assignment
- **WHEN** the supertagger assigns `dir · dir^l` to chunk "do not" with negation voiding
- **THEN** the TypeAssignment SHALL have `voiding: Some(Negation)`

### Requirement: Input validation
Before flattening, the parser SHALL validate the input:
1. `chunk_idx` values SHALL be monotonically non-decreasing across the input sequence.
2. No `TypeExpr` SHALL be empty (length 0).

If validation fails, the parser SHALL return a `LinkageGraph` with zero meta entries, zero edges, and `timed_out: false`.

#### Scenario: Valid input accepted
- **WHEN** given assignments with chunk_idx values `[0, 0, 1, 2]`
- **THEN** validation passes and parsing proceeds

#### Scenario: Non-monotonic chunk_idx rejected
- **WHEN** given assignments with chunk_idx values `[0, 2, 1]`
- **THEN** the parser SHALL return an empty `LinkageGraph`

#### Scenario: Empty TypeExpr rejected
- **WHEN** given an assignment with `TypeExpr(vec![])`
- **THEN** the parser SHALL return an empty `LinkageGraph`

### Requirement: Flattened type sequence
The parser SHALL flatten all TypeExprs into a single `Vec<SimpleType>` for reduction, recording a parallel `Vec<u16>` of chunk indices.

#### Scenario: Flatten and parse
- **WHEN** given assignments `[(0, dir · ag^l, None), (1, ag, None)]`
- **THEN** the parser SHALL flatten to `[dir, ag^l, ag]` with chunk indices `[0, 0, 1]`

### Requirement: Conjunction as segment barrier
Before running the DP algorithm, the parser SHALL split the flattened sequence into segments at every position `i` where `meta[i].simple_type.base == TypeId::Conj`. The `conj` position itself is excluded from all segments. Non-conj positions from a multi-element `conj` TypeExpr join the adjacent segment.

No contraction edge SHALL cross a `conj` position.

#### Scenario: Conjunction splits segments
- **WHEN** parsing `[dir, ag^l, ag, conj, dir, usr^l, usr]`
- **THEN** the parser SHALL process positions 0..2 and 4..6 as independent segments, with position 3 excluded

#### Scenario: No cross-conjunction contraction
- **WHEN** `ag^l` is in segment 1 and `ag` is in segment 2 (across a `conj`)
- **THEN** the parser SHALL NOT contract them

#### Scenario: Multi-element conj chunk
- **WHEN** a chunk has TypeExpr `conj · n^r` flattened to positions `[conj(3), n^r(4)]`
- **THEN** position 3 is excluded (barrier), position 4 joins the following segment

### Requirement: Nussinov DP algorithm (Pass 1)
The system SHALL implement the Nussinov dynamic programming algorithm to find the maximum number of non-crossing contraction edges within each segment. The algorithm SHALL have O(n³) time complexity where n is the segment length.

The recurrence for segment `seq[lo..=hi]`:
```
dp[i][j] = maximum number of non-crossing edges in subsequence i..=j

Base cases:
    dp[i][i] = 0
    dp[i][j] = 0 if j < i

Recurrence:
    dp[i][j] = max(
        dp[i+1][j],                                            // seq[i] unmatched
        max over k in (i+1..=j) where can_contract(seq[i], seq[k]):
            dp[i+1][k-1] + dp[k+1][j] + 1                     // seq[i] matches seq[k]
    )
```

Backpointers SHALL be stored alongside each `dp[i][j]` entry to record which choice (unmatched, or matched with which `k`) produced the optimal value.

#### Scenario: All potential match partners considered
- **WHEN** processing `[a^l, b^l, b, a]` (positions 0..3)
- **THEN** the algorithm SHALL consider matching position 0 with position 3 (`a^l ↔ a`, nesting `b^l ↔ b` inside), not just boundary split points

#### Scenario: Nested contractions found
- **WHEN** parsing `[a^l, b^l, b, a]`
- **THEN** the parser SHALL find both edges `(0, 3)` and `(1, 2)` — the maximal planar linkage

### Requirement: Linkage extraction via backpointers
After DP completion, the parser SHALL extract the concrete set of edges by walking the backpointer table starting from `dp[0][seg_len-1]` for each segment. This is standard DP backtrace.

The DP table uses **segment-local** indices (0-based within each segment). During extraction, each segment-local index SHALL be translated to a **global** flattened-sequence position by adding the segment's starting offset. All `LinkageEdge` values in the output use global positions.

#### Scenario: Extraction with index translation
- **WHEN** segment starting at global offset 5 has local `dp[0][3] = 2` with backpointers indicating local matches at (0,3) and (1,2)
- **THEN** the extraction SHALL produce `LinkageEdge { left: 5, right: 8 }` and `LinkageEdge { left: 6, right: 7 }`

### Requirement: Security-aware second pass (Pass 2)
After Pass 1 extraction, the parser SHALL scan the extracted linkage for injection-relevant edges. An edge `(i, j)` is **injection-relevant** if:
- `seq[i].base` or `seq[j].base` is `Ag`, AND
- The other endpoint shares a `chunk_idx` with a node whose base is `Dir` or `Role`.

If Pass 1's linkage contains zero injection-relevant edges, the parser SHALL run a second Nussinov pass with a modified scoring function:
```
score(i, k) = dp_bonus + dp[i+1][k-1] + dp[k+1][j] + 1
where dp_bonus = n if edge (i, k) is injection-relevant, else 0
```
(where `n` is the segment length, ensuring any injection-relevant edge outweighs all non-injection edges.)

If Pass 2 produces a linkage with injection-relevant edges, it SHALL be returned instead of Pass 1's result.

#### Scenario: Pass 1 suffices
- **WHEN** Pass 1's maximal linkage already contains an injection-relevant edge
- **THEN** Pass 2 SHALL NOT run; Pass 1's result is returned

#### Scenario: Pass 2 surfaces hidden injection
- **WHEN** Pass 1's linkage has 5 total edges but 0 injection-relevant, and an alternative planar linkage has 3 edges including 1 injection-relevant edge
- **THEN** Pass 2 SHALL find and return the alternative linkage

#### Scenario: No injection exists
- **WHEN** no valid planar linkage contains any injection-relevant edges
- **THEN** Pass 2 SHALL return the same result as Pass 1 (max total contractions)

### Requirement: Planar reduction constraint
All edges in the extracted linkage SHALL be non-crossing: for any two edges (a,b) and (c,d), it SHALL NOT be the case that a < c < b < d.

#### Scenario: Reject crossing reduction
- **WHEN** a potential reduction would require position 0 to match position 3, while position 1 matches position 2 in a crossing pattern
- **THEN** the Nussinov recurrence inherently prevents this

#### Scenario: Nested planar reduction
- **WHEN** positions 1 and 2 match, and positions 0 and 3 match around them
- **THEN** this is valid (nested, non-crossing)

### Requirement: LinkageGraph output
The system SHALL produce a `LinkageGraph` containing:
- `meta: Vec<NodeMeta>` — one entry per position in the flattened sequence. `meta[i]` records `chunk_idx` and `simple_type` for position `i`.
- `edges: Vec<LinkageEdge>` — the non-crossing contraction edges, sorted by `left`.
- `timed_out: bool` — whether the parse was interrupted by timeout.

Node identity is the vector index — no separate `flat_idx` field.

#### Scenario: Simple contraction
- **WHEN** parsing `[ag^l, ag]` which contracts
- **THEN** the graph SHALL contain 2 meta entries and 1 edge `(0, 1)`

#### Scenario: Multiple contractions
- **WHEN** parsing `[dir, ag^l, ag, usr^l, usr]`
- **THEN** the graph SHALL contain 5 meta entries and edges `(1, 2)` and `(3, 4)`

### Requirement: Chunk boundary tracking
Each `NodeMeta` entry SHALL record the `chunk_idx` of the chunk it originated from. This enables the scope checker to determine same-chunk adjacency.

#### Scenario: Chunk provenance preserved
- **WHEN** chunk 0 has TypeExpr `dir · ag^l` (flattened to positions 0 and 1)
- **THEN** `meta[0].chunk_idx == 0` and `meta[1].chunk_idx == 0`

### Requirement: Parse always returns a result
The parser SHALL never fail. It always produces a `LinkageGraph`. The graph may contain zero edges.

#### Scenario: No contractions possible
- **WHEN** parsing `[dir, usr, n]`
- **THEN** the parser SHALL return a `LinkageGraph` with 3 meta entries and 0 edges

#### Scenario: Empty input
- **WHEN** parsing an empty sequence
- **THEN** the parser SHALL return a `LinkageGraph` with 0 meta entries and 0 edges

### Requirement: Parse timeout
The parser SHALL accept an optional timeout duration. If the timeout is exceeded during Pass 1 or Pass 2, the parser SHALL stop and return the best linkage found so far, with `timed_out: true`.

#### Scenario: Parse within timeout
- **WHEN** parsing completes before the timeout
- **THEN** the result SHALL have `timed_out: false`

#### Scenario: Parse exceeds timeout
- **WHEN** parsing exceeds the timeout during Pass 1
- **THEN** the result SHALL have `timed_out: true` and contain a best-effort linkage
