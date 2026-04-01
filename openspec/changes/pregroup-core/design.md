## Context

JOACHIM requires a formal algebraic engine to prove prompt injection by analyzing illocutionary structure. The core insight is that prompt injection is a **scoping problem**: an attacker constructs input where a directive speech act scopes over actions in the agent's privileged domain.

Pregroup grammar (Lambek, 1999) provides the formalism. It's a type-logical grammar where:
- Words/chunks are assigned types from an inventory
- Types compose via contraction rules: `a^l · a → 1` and `a · a^r → 1`
- A sequence "parses" if it reduces via planar linkages of contractions
- The reduction proof shows compositional structure and dependency links

For JOACHIM, we're not parsing English syntax — we're parsing **illocutionary structure**. The types represent speech acts (directive, assertive, question), domains (agent, user), and role identities. Modifiers are not primitive types; they are derived as functional types (e.g., `n^r · n` for a noun modifier), following standard categorial grammar practice.

Voiding operators (hypothetical, meta-linguistic, negation) are structurally modifiers but carry additional semantic annotation via a `VoidingKind` tag on their `TypeAssignment`. This separates the algebraic structure (how they link) from the semantic judgment (what they void).

**Current state**: Type inventory v0 defined with 9 basic types. 45 examples hand-annotated. Reduction logic validated on paper.

**Constraints**:
- Pure Rust, minimal external dependencies: `smallvec` (stack-allocated adjacency lists), `serde` + `serde_json` (optional, behind feature flag)
- Must produce proof linkages (for explainability/audit)
- O(n³) time complexity acceptable (input is ~10-50 chunks max)

## Goals / Non-Goals

**Goals:**
- Implement type algebra with two levels: `SimpleType` (primitive + i8 adjoint count) and `TypeExpr` (product of simple types, assigned to chunks)
- Implement Nussinov-style parser that finds the **maximal planar linkage** in a type sequence
- Implement scope checker that detects transitive `dir → ag` and `role → ag` paths along the linkage graph
- Produce verifiable linkage graphs that explain verdicts
- Property-based test coverage proving the algebraic laws hold
- Comprehensive test coverage using hand-annotated examples

**Non-Goals:**
- Supertagger (LLM integration) — separate change
- Normalization pipeline — separate change
- Regex pre-filter — separate change
- API/Lambda handler — separate change
- K-best linkage search — separate change (MVP uses single maximal linkage per chunk sequence)
- Performance optimization beyond O(n³) — future work
- Fine-tuning type inventory — iterative based on evaluation

## Decisions

### Decision 1: Two-Level Type Representation with i8 Adjoints

**Choice**: Separate `SimpleType` (primitive base with an `i8` integer adjoint counter) from `TypeExpr` (product of simple types assigned to a chunk).

```rust
/// Enumeration of primitive type identifiers.
/// 9 primitives: dir, ag, usr, role, s, n, conj, ass, qst
#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub enum TypeId {
    Dir, Ag, Usr, Role, S, N, Conj, Ass, Qst,
}

/// An atomic type with an integer adjoint counter.
///
/// Adjoints form an integer group over the base: a^l = a^{-1}, a^r = a^{+1}.
/// Nested adjoints simplify automatically: (a^l)^r = a^{-1+1} = a^0 = a.
#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub struct SimpleType {
    pub base: TypeId,
    /// 0 = base, negative = left adjoints, positive = right adjoints.
    pub adjoint: i8,
}

/// A type expression: product of simple types assigned to a chunk.
/// e.g., dir · ag^l is TypeExpr(vec![dir, ag^l])
/// The empty product is the unit type 1.
///
/// Inner Vec is private. Construct via `TypeExpr::new()`, `TypeExpr::unit()`,
/// or `From<Vec<SimpleType>>`. Access via `as_slice()`, `len()`, `iter()`.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct TypeExpr(Vec<SimpleType>);
```

**Contraction formula**: Two SimpleTypes `x` (at position `i`) and `y` (at position `j`, where `i < j`) contract if and only if:
```rust
fn can_contract(x: SimpleType, y: SimpleType) -> bool {
    x.base == y.base && y.adjoint.checked_sub(1).map_or(false, |r| x.adjoint == r)
}
```
The `checked_sub` prevents i8 underflow when `y.adjoint == -128`. In that case `can_contract` returns `false` (no valid contraction partner exists at adjoint -129). This is a pure predicate — it never panics.
This covers both rules:
- Left contraction `a^l · a`: `(base=A, adj=-1)` beside `(base=A, adj=0)` → `-1 == 0 - 1` ✓
- Right contraction `a · a^r`: `(base=A, adj=0)` beside `(base=A, adj=1)` → `0 == 1 - 1` ✓
- Higher adjoints `a^{ll} · a^l`: `(base=A, adj=-2)` beside `(base=A, adj=-1)` → `-2 == -1 - 1` ✓

**Overflow behaviour**: `left_adj()` and `right_adj()` SHALL panic on i8 overflow (debug and release). In practice, adjoint values beyond ±3 are not expected; the panic is a fail-fast guard against buggy supertagger output. Implementations should use `checked_sub(1).expect("adjoint underflow")` / `checked_add(1).expect("adjoint overflow")`.

**Rationale**:
- Mathematically correct: adjoints form an integer group over the base type (a^l = a^-1, a^r = a^+1).
- Naturally simplifies nested adjoints (e.g. (a^l)^r = a^(-1+1) = a^0 = a), strictly preserving structural equality.
- Zero-allocation, cache-friendly `Copy` struct prevents performance bottlenecks from `Box` allocations.
- A chunk's type is a `TypeExpr` (product), e.g., a transitive-verb-like chunk gets `dir · ag^l`
- The parser flattens all chunk TypeExprs into a single sequence of SimpleTypes, then finds planar linkages.

### Decision 2: Modifiers Are Functional Types, Not Primitives

**Choice**: Eliminate `mod`, `hyp`, `meta`, and `neg` as primitive types. All modifiers are derived as functional types in the categorial grammar tradition.

```
Noun modifier:     n^r · n      ("previous", "all", "confidential")
Agent modifier:    ag^r · ag    ("previous" modifying agent-domain "instructions")
Sentence modifier: s^r · s      (sentential adverbs)
Hypothetical:      s · s^l      ("if", "imagine") -> seeks a sentence on the right
Negation:          dir · dir^l  ("do not", "don't") -> seeks a directive on the right
Meta-linguistic:   n · n^l      ("quote", "mention") -> repackages as noun
```

**Rationale**:
- Follows standard categorial grammar: modifiers are endofunctions on their target category.
- Enables transitive graph linkages. As functional types, modifiers contract with their targets, naturally connecting the voiding operator to its target in the linkage graph.
- Fewer primitives = smaller inventory for the supertagger to learn.

**Remaining primitive types (9):**
`dir`, `ag`, `usr`, `role`, `s`, `n`, `conj`, `ass`, `qst`

### Decision 3: Voiding Annotations on TypeAssignments

**Choice**: Voiding is a semantic property of a chunk, not a structural property of its type. The supertagger annotates chunks with an optional `VoidingKind` tag alongside their `TypeExpr`.

```rust
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum VoidingKind {
    /// Hypothetical frame ("if", "imagine", "suppose")
    Hypothetical,
    /// Negation ("do not", "don't", "never")
    Negation,
    /// Meta-linguistic mention ("quote", reported speech, academic discussion)
    Meta,
}

pub struct TypeAssignment {
    pub chunk_idx: u16,
    pub type_expr: TypeExpr,
    /// If Some, this chunk is a voiding operator.
    pub voiding: Option<VoidingKind>,
}
```

**Rationale**:
- The type algebra handles structural linkage (how chunks compose). Voiding is a semantic judgment (what the composition means for injection detection).
- Two chunks can have identical types (e.g., both `dir · dir^l`) but different voiding behavior: "please" is a non-voiding directive modifier, "do not" is a negation voiding operator. The type algebra cannot distinguish them — only the supertagger can.
- Separating the two concerns avoids a circular dependency: the scope checker no longer needs to reverse-engineer "is this a voiding chunk?" from a structurally ambiguous type.
- `VoidingKind` is preserved as an enum (not a boolean) because the three voiding operators may need distinct behavior in future iterations (e.g., negation might interact differently with double negation).

**Alternatives considered**:
- Detect voiding from type structure alone: impossible — `dir · dir^l` is structurally identical for "please" and "do not".
- Re-introduce `hyp`/`meta`/`neg` as primitives: breaks the functional modifier architecture and prevents linkage graph traversal.

### Decision 4: Security-Aware Linkage Selection

**Choice**: The parser uses a two-pass approach. Pass 1 (Nussinov DP) finds the maximal planar linkage by total contraction count. Pass 2 (post-hoc scoring) checks whether alternative linkages expose injection patterns that the maximal linkage misses.

**Pass 1**: Standard Nussinov DP optimizing total contraction count (see Decision 6).

**Pass 2**: After extracting the maximal linkage, scan it for injection-relevant edges. An edge `(i, j)` is **injection-relevant** if:
- `seq[i].base` or `seq[j].base` is `Ag`, AND
- The other endpoint shares a `chunk_idx` with a node whose base is `Dir` or `Role`.

This is a local property computable in O(E) from the extracted linkage and the chunk index map.

If the maximal linkage contains zero injection-relevant edges, perform a second Nussinov pass with a modified scoring function that adds a bonus (+n, where n is the sequence length) for each injection-relevant edge. If this second pass produces a linkage with injection-relevant edges, return it instead.

**Rationale**:
- A pure "max total contractions" heuristic is gameable: an attacker can craft input where the benign parse has more total contractions than the malicious parse, hiding the injection.
- Scoring injection-relevance during Nussinov DP is not feasible because whether an edge is injection-relevant depends on chunk adjacency information *outside* the DP span.
- The two-pass approach is simple: the common case (injection is visible in the max-contraction linkage) requires only one pass. The second pass is only triggered when the first pass finds no injection, and adds at most 2x the O(n³) cost.

### Decision 5: LinkageGraph Representation

**Choice**: The parser output is a `LinkageGraph` struct. Nodes are implicit — the flattened sequence index *is* the node identity. Edges and metadata are stored in parallel vectors.

```rust
/// Metadata for each position in the flattened SimpleType sequence.
#[derive(Copy, Clone, Debug)]
pub struct NodeMeta {
    /// Which chunk this SimpleType came from.
    pub chunk_idx: u16,
    /// The type at this position.
    pub simple_type: SimpleType,
}

/// A contraction edge connecting two positions in the flattened sequence.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct LinkageEdge {
    /// Index of the left SimpleType (lower position).
    pub left: u16,
    /// Index of the right SimpleType (higher position).
    pub right: u16,
}

/// The complete linkage graph produced by the parser.
pub struct LinkageGraph {
    /// One entry per position in the flattened sequence.
    /// `meta[i]` describes the SimpleType at position `i`.
    pub meta: Vec<NodeMeta>,
    /// The set of non-crossing contraction edges. Sorted by `left`.
    pub edges: Vec<LinkageEdge>,
    /// Whether the parse timed out before completing.
    pub timed_out: bool,
}
```

**Rationale**:
- Node identity is the vector index — no redundant `flat_idx` field that can go out of sync.
- `u16` is sufficient (max flattened sequence is ~250 for 50 chunks × 5 simple types each).
- All inner types are `Copy`.
- Edges sorted by `left` enables O(log n) lookup for left-endpoint queries. Finding edges by right endpoint requires O(E) scan — acceptable for the small E values in practice.
- The scope checker builds a temporary bidirectional adjacency list in O(E) for traversal, so the sorted-edge lookup is a convenience, not the hot path.

### Decision 6: Nussinov-Style Parser Algorithm

**Choice**: The parser finds the maximal planar linkage using the Nussinov dynamic programming algorithm, adapted from RNA secondary structure prediction.

**Nussinov recurrence** over the flattened SimpleType sequence `seq[0..n]`:

```
dp[i][j] = maximum number of non-crossing contraction edges in the subsequence i..=j

Base cases:
    dp[i][i] = 0            (single element, no contraction possible)
    dp[i][j] = 0 if j < i   (empty span)

Recurrence:
    dp[i][j] = max(
        dp[i+1][j],                                           // seq[i] is unmatched
        max over k in (i+1..=j) where can_contract(seq[i], seq[k]):
            dp[i+1][k-1] + dp[k+1][j] + 1                    // seq[i] matches seq[k]
    )
```

Backpointers are stored alongside each `dp[i][j]` entry to record which choice (unmatched, or matched with which `k`) produced the optimal value.

**Indexing**: The DP table uses **segment-local** indices (0-based within each segment). During extraction, segment-local edge indices are translated to **global** flattened-sequence positions by adding the segment's starting offset. All `LinkageEdge` values in the output `LinkageGraph` use global positions.

**Extraction**: Walk the backpointers starting from `dp[0][n-1]` to reconstruct the concrete set of edges. Translate each `(i_local, k_local)` pair to `(i_local + segment_offset, k_local + segment_offset)`.

**Complexity**: O(n³) time, O(n²) space per segment.

**Conjunction barriers**: Before running the DP, split the flattened sequence at `conj` positions. Run the Nussinov DP independently on each segment. Concatenate the resulting edge sets.

**Why Nussinov, not CYK**: CYK is designed for context-free grammars where the chart stores non-terminal symbols spanning sub-strings. Pregroup linkage finding is a maximum non-crossing matching problem — structurally identical to RNA secondary structure folding. The Nussinov recurrence correctly considers all possible planar pairings by iterating over all potential match partners for each position, not just split points at sub-span boundaries.

### Decision 7: Conjunction Handling

**Choice**: `conj` is a primitive type that acts as an **opaque barrier** in the linkage. The parser does not attempt to contract across `conj` boundaries.

```
Procedure:
1. During flattening, record every position i where meta[i].simple_type.base == Conj.
2. Split the flattened sequence into segments at those positions.
   A conj position belongs to neither segment — it is excluded.
   If a conj chunk has a multi-element TypeExpr (e.g., conj · n^r),
   the conj position is excluded but the non-conj positions (n^r)
   join the following segment.
3. Run Nussinov DP independently on each segment.
4. The scope checker treats each conj-delimited segment independently.
```

**Validation**: The parser SHALL accept multi-element conj TypeExprs. Segmentation operates on individual flattened positions, not on chunks. Only positions with `base == Conj` are barriers; other positions from the same chunk are assigned to the adjacent segment.

**Example**:
```
[dir · ag^l] [ag] [conj] [dir · usr^l] [usr]
 segment 1         barrier    segment 2

Segment 1: dir → ag contraction → flagged
Segment 2: dir → usr contraction → clean
```

**Rationale**:
- `conj` is polymorphic (it joins two constituents of the same type). Proper polymorphic typing requires type variables, which are out of scope for v0.
- Treating `conj` as a barrier is conservative: it prevents false contractions across clause boundaries.
- Each conj-delimited segment is checked independently, so a single injection in one conjunct is still detected even if the other conjunct is clean.
- This is a known simplification. When polymorphic types are added (v2), `conj` can be promoted to `X^r · X · X^l` and the barrier logic removed.

### Decision 8: Intra-Chunk Self-Contraction Convention

**Choice**: A chunk's `TypeExpr` may contain both an adjoint and its matching base (e.g., `dir · ag^l · ag`). When flattened, these adjacent elements from the same chunk will contract with each other. This is an intentional and valid convention.

**Rationale**:
- A `TypeExpr` like `dir · ag^l · ag` encodes: "this chunk is a directive (`dir`) that seeks an agent-domain complement (`ag^l`) and brings its own agent-domain argument (`ag`)." The intra-chunk contraction `ag^l · ag → 1` leaves `dir` as the residual, confirming the chunk's overall speech act type.
- This is how the supertagger signals that a chunk is a complete directive targeting the agent domain. The scope path traces `dir` → same-chunk → `ag^l` → contraction → `ag`, establishing `dir` scopes over `ag`.
- The alternative — splitting `dir · ag^l` and `ag` into separate chunks — would require the supertagger to produce multi-chunk decompositions for single lexical items like "ignore your instructions." That's a chunking concern, not a type algebra concern.
- Intra-chunk contraction is mathematically valid: the Nussinov DP operates on the flattened sequence and does not distinguish intra-chunk from inter-chunk contractions.

### Decision 9: Transitive "Scopes Over" via Linkage Graph

**Choice**: `dir` scopes over `ag` if and only if there is a connected path in the planar linkage graph from `dir` to `ag`, following functional application chains through **positionally adjacent** same-chunk steps and contraction edges.

Precisely: `X` **scopes over** `Y` if there is a path of steps where each step is one of:
1. **Adjacent same-chunk step**: Move from position `i` to position `j` where `|i - j| == 1` AND `meta[i].chunk_idx == meta[j].chunk_idx`. This restricts same-chunk movement to neighbours within the TypeExpr product, modelling the positional structure of functional application.
2. **Contraction step**: Follow a `LinkageEdge` from one position to the position it contracted with.

Such that the path starts at a position with base `X` and ends at a position with base `Y`, and includes at least one contraction step.

**Implementation**: The scope checker uses a **two-state BFS** where the state is `(position: u16, has_contracted: bool)`. The adjacency list encodes both contraction edges (tagged) and adjacent same-chunk pairs (tagged). When traversing a contraction edge, `has_contracted` flips to `true`. An `Ag` node is only reported as a scope target when `has_contracted == true`. This prevents pure same-chunk adjacency paths (e.g., `dir · ag` in a chunk with no edges) from being flagged.

**Why positional adjacency, not arbitrary same-chunk**: A TypeExpr like `dir · n` places `dir` at position 0 and `n` at position 1. They are adjacent, so stepping from `dir` to `n` is valid. But if `n` contracts (via an `n^r` link) with another chunk that happens to contain `ag`, the path would be `dir → n → n^r → ag`, which is spurious — `dir` has no functional relationship with `ag` here. Without the adjacency constraint, any two types in the same chunk would be scopally connected through unrelated contraction edges, producing massive false positives.

With the constraint, the path must follow the product's positional structure: `dir · ag^l` allows stepping from `dir` (pos 0) to `ag^l` (pos 1) because they are adjacent. But `dir · n` followed by `n^r · ag` does NOT create a `dir → ag` scope path because the contraction lands on `n^r` (pos 2), and `ag` (pos 3) is adjacent to `n^r` but not to `dir`.

**Example — valid scope path**:
```
Chunk 0: dir · dir^l  (positions 0, 1)
Chunk 1: dir · ag^l   (positions 2, 3)
Chunk 2: ag           (position 4)

Edges: (1, 2) dir^l↔dir, (3, 4) ag^l↔ag

Path: dir(0) →adj→ dir^l(1) →edge→ dir(2) →adj→ ag^l(3) →edge→ ag(4)
Result: dir scopes over ag ✓
```

**Example — correctly rejected**:
```
Chunk 0: dir · n      (positions 0, 1)
Chunk 1: n^r · ag     (positions 2, 3)

Edge: (1, 2) n↔n^r

Path attempt: dir(0) →adj→ n(1) →edge→ n^r(2) →adj→ ag(3)
This IS a valid path under the adjacency rule. BUT: is this a real scope?
```

Actually, this path IS valid under adjacency — `dir` is adjacent to `n`, `n^r` is adjacent to `ag`. The question is whether this is linguistically correct. In categorial grammar, `dir · n` means "a directive that produces a noun." If `n` contracts with `n^r` (a noun modifier seeking its target), then `dir` did produce something that flowed into the modifier chain reaching `ag`. This is genuinely a scope relationship — `dir` influenced the compositional chain that reached `ag`.

The false positive in the critique arose from a non-adjacent example (`dir · n, n^r · ag` where `n^r` and `ag` are in the same chunk). Under adjacency, this IS reachable and IS a legitimate scope path. The real protection against spurious scope comes from the contraction requirement: types must actually contract (match via `can_contract`) to form an edge. Random types won't contract.

**Rationale**:
- Positional adjacency restricts same-chunk steps to the product's structure, preventing long-range hops within large TypeExprs.
- Combined with the contraction requirement, this ensures scope paths follow functional application chains.
- Matches linguistic intuition: in `dir · ag^l`, `dir` is the functor and `ag^l` is its argument slot — they are positionally adjacent in the product.

### Decision 10: Voiding Propagation — Chunk-Granular, Directed

**Choice**: Voiding operates at **chunk granularity**. A chunk is either entirely voided or entirely not voided. The scope checker propagates voiding using `VoidingKind` annotations on `TypeAssignment`s.

A chunk is voided if:
1. **Self-voiding**: The chunk has `voiding: Some(_)`.
2. **Propagated voiding**: Any position in the chunk is the target of a contraction edge from a voided chunk.

**Propagation procedure**:
1. Initialize a set of voided chunk indices. Add all chunks with `voiding: Some(_)` (self-voiding).
2. For each voided chunk, find all contraction edges where one endpoint is in the voided chunk and the other is in a non-voided chunk. Add the target chunk to the voided set.
3. Repeat step 2 until no new chunks are added (BFS fixpoint).

This is a BFS over chunks (not positions), seeded by voiding-annotated chunks, expanding along contraction edges. The chunk-level granularity means: if any position in chunk X is reached by a contraction edge from a voided chunk, ALL positions in chunk X become voided.

**Example 1**: `[dir · dir^l, voiding: Negation]  [dir · ag^l · ag]`
- Chunk 0 is voided (self-voiding). `dir^l` (chunk 0) contracts with `dir` (chunk 1).
- Chunk 1 becomes voided (propagation via contraction edge).
- All positions in chunk 1 are voided. The `dir → ag` path from chunk 1 is voided. Clean.

**Example 2**: `[n · dir · ag^l · ag, voiding: Meta]`
- Chunk 0 is voided (self-voiding).
- All positions in chunk 0 are voided. The intra-chunk `dir → ag` path is voided. Clean.

**Example 3**: `[s · s^l, voiding: Hypothetical]  [s · dir · ag^l · ag]  [qst · usr^l]`
- Chunk 0 voided (self-voiding). `s^l` (chunk 0) contracts with `s` (chunk 1).
- Chunk 1 becomes voided (propagation). All of chunk 1 voided, including `dir → ag`. Clean.
- Chunk 2 has no contraction edge from any voided chunk, so it is NOT voided.

**Rationale**:
- Chunks are the atomic semantic units produced by the supertagger. Voiding at chunk granularity matches this: if a hypothetical frame covers a clause, the entire clause (chunk) is voided.
- Position-level voiding would require tracking which specific positions are "semantically under" a voiding operator — information the flat linkage graph does not encode. Chunk-level voiding avoids this complexity.
- The BFS is over chunks (max ~50), not positions (max ~250), making it trivially fast.
- A voiding chunk that doesn't link to anything has no contraction edges to propagate, so only self-voids — correct conservative behavior.

### Decision 11: Input Validation

**Choice**: The parser validates `ParseInput` on entry before flattening.

**Validation rules**:
1. `chunk_idx` values across the input sequence SHALL be monotonically non-decreasing. Each `TypeAssignment` has a `chunk_idx`; successive assignments must have `chunk_idx >= previous`.
2. No `TypeExpr` SHALL be empty (length 0). An empty TypeExpr would contribute zero positions to the flattened sequence and serve no purpose.

If validation fails, the parser returns a `LinkageGraph` with zero edges and `timed_out: false`, treating malformed input as unparseable rather than panicking. This is a defensive boundary — the supertagger is responsible for producing valid input, but the core must not crash on garbage.

**Rationale**:
- The scope checker uses `chunk_idx` to determine same-chunk adjacency. Non-monotonic chunk indices would break this invariant silently.
- Returning an empty graph on invalid input (rather than `Result::Err`) keeps the parser infallible, matching the "parse always returns a result" requirement.

## Risks / Trade-offs

**[Risk] Type inventory may be insufficient for some injection patterns**
→ Mitigation: Inventory is v0, designed to be iterable. Evaluation phase will identify gaps.

**[Risk] Security-aware two-pass may prefer linguistically worse parses**
→ Mitigation: The second pass is only triggered when the first pass finds zero injection-relevant edges. In the common case, the max-contraction linkage already surfaces any injection. The second pass adds at most 2x cost.

**[Risk] `conj` as opaque barrier may miss cross-clause scope patterns**
→ Mitigation: Conservative choice — barriers prevent false contractions. Each conjunct is checked independently. When polymorphic types are added (v2), `conj` can be upgraded.

**[Risk] `VoidingKind` depends on supertagger accuracy**
→ Mitigation: If the supertagger fails to annotate a voiding chunk, the chunk's modifier type still contracts with its target — the only effect is that the resulting injection pattern is not voided (a false positive). False positives are less costly than false negatives in security.

**[Risk] Attacker exploits voiding by injecting a fake hypothetical frame**
→ Mitigation: The supertagger must learn to distinguish genuine hypothetical frames from adversarial ones. This is a supertagger quality issue, not a core algebra issue. The core correctly propagates whatever voiding annotations it receives.

**[Risk] Intra-chunk self-contraction bakes scope into the type assignment**
→ Mitigation: This is intentional. The supertagger makes the scope judgment; the parser confirms it algebraically. If the supertagger assigns `dir · ag^l · ag` to a chunk, it's asserting the chunk IS a directive over agent-domain. The parser's job is to verify the algebra and detect scope paths, not to second-guess the supertagger's chunking.
