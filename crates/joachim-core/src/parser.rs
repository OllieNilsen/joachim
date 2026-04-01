//! Nussinov-style parser for pregroup type sequences.
//!
//! The parser flattens a sequence of [`TypeAssignment`]s into a flat
//! `SimpleType` sequence, splits at conjunction barriers, runs the Nussinov
//! DP to find the maximal planar linkage, and optionally runs a security-aware
//! second pass if no injection-relevant edges were found.
//!
//! The parser is **infallible**: it always returns a [`LinkageGraph`]. Invalid
//! input produces an empty graph. Timeout produces a best-effort graph with
//! `timed_out: true`.

use std::time::{Duration, Instant};

use crate::linkage::{LinkageEdge, LinkageGraph, NodeMeta};
use crate::types::{can_contract, SimpleType, TypeAssignment, TypeId};

// ---------------------------------------------------------------------------
// ParseInput
// ---------------------------------------------------------------------------

/// Input to the parser: a sequence of type assignments from the supertagger.
#[derive(Clone, Debug)]
pub struct ParseInput(pub Vec<TypeAssignment>);

// ---------------------------------------------------------------------------
// Validation
// ---------------------------------------------------------------------------

/// Validate parse input.
///
/// Rules:
/// 1. `chunk_idx` values must be monotonically non-decreasing.
/// 2. No `TypeExpr` may be empty.
fn validate(input: &ParseInput) -> bool {
    let mut prev_chunk: Option<u16> = None;
    for ta in &input.0 {
        if ta.type_expr.is_empty() {
            return false;
        }
        if let Some(prev) = prev_chunk {
            if ta.chunk_idx < prev {
                return false;
            }
        }
        prev_chunk = Some(ta.chunk_idx);
    }
    true
}

// ---------------------------------------------------------------------------
// Flatten
// ---------------------------------------------------------------------------

/// Flatten type assignments into a flat SimpleType sequence with parallel
/// chunk index vector.
fn flatten(input: &ParseInput) -> (Vec<SimpleType>, Vec<u16>) {
    let mut types = Vec::new();
    let mut chunks = Vec::new();
    for ta in &input.0 {
        for &st in ta.type_expr.as_slice() {
            types.push(st);
            chunks.push(ta.chunk_idx);
        }
    }
    (types, chunks)
}

// ---------------------------------------------------------------------------
// Conjunction barrier detection
// ---------------------------------------------------------------------------

/// Find segment boundaries by splitting at `Conj` positions.
///
/// Returns a list of `(start, end)` pairs (inclusive start, exclusive end)
/// for each non-empty segment. `Conj` positions are excluded from all
/// segments. Non-conj positions from multi-element conj chunks join the
/// adjacent segment.
fn find_segments(types: &[SimpleType]) -> Vec<(usize, usize)> {
    let mut segments = Vec::new();
    let mut seg_start: Option<usize> = None;

    for (i, st) in types.iter().enumerate() {
        if st.base == TypeId::Conj {
            // Close current segment if open.
            if let Some(start) = seg_start.take() {
                segments.push((start, i));
            }
        } else if seg_start.is_none() {
            seg_start = Some(i);
        }
    }
    // Close final segment.
    if let Some(start) = seg_start {
        segments.push((start, types.len()));
    }
    segments
}

// ---------------------------------------------------------------------------
// Nussinov DP
// ---------------------------------------------------------------------------

/// Backpointer: records the decision made for dp[i][j].
#[derive(Copy, Clone, Debug)]
enum Backptr {
    /// Position i was unmatched; solution is dp[i+1][j].
    Unmatched,
    /// Position i matched with position k (segment-local index).
    Matched { k: usize },
}

/// Run the Nussinov DP on a segment, returning (dp table, backpointer table).
///
/// `seq` is the segment slice. `bonus_fn` is called for each candidate edge
/// `(i, k)` (segment-local) and returns a bonus score (0 for Pass 1, n for
/// injection-relevant edges in Pass 2).
fn nussinov_dp(
    seq: &[SimpleType],
    bonus_fn: &dyn Fn(usize, usize) -> usize,
    deadline: Option<Instant>,
) -> (Vec<Vec<usize>>, Vec<Vec<Backptr>>, bool) {
    let n = seq.len();
    let mut dp = vec![vec![0usize; n]; n];
    let mut bp = vec![vec![Backptr::Unmatched; n]; n];
    let mut timed_out = false;

    // Fill diagonals from short spans to long spans.
    for span in 1..n {
        for i in 0..n - span {
            let j = i + span;

            // Check timeout periodically (every row of the outer loop).
            if let Some(dl) = deadline {
                if Instant::now() >= dl {
                    timed_out = true;
                    return (dp, bp, timed_out);
                }
            }

            // Option 1: i is unmatched.
            let mut best = dp[i + 1][j];
            let mut best_bp = Backptr::Unmatched;

            // Option 2: i matches some k in (i+1..=j).
            for k in (i + 1)..=j {
                if can_contract(seq[i], seq[k]) {
                    let left_score = if i + 1 < k { dp[i + 1][k - 1] } else { 0 };
                    let right_score = if k < j { dp[k + 1][j] } else { 0 };
                    let bonus = bonus_fn(i, k);
                    let score = left_score + right_score + 1 + bonus;

                    if score > best {
                        best = score;
                        best_bp = Backptr::Matched { k };
                    }
                }
            }

            dp[i][j] = best;
            bp[i][j] = best_bp;
        }
    }

    (dp, bp, timed_out)
}

/// Extract edges from backpointer table. Returns segment-local edge pairs.
fn extract_edges(bp: &[Vec<Backptr>], i: usize, j: usize, edges: &mut Vec<(usize, usize)>) {
    if i >= j || i >= bp.len() || j >= bp[0].len() {
        return;
    }
    match bp[i][j] {
        Backptr::Unmatched => {
            // i is unmatched; recurse on (i+1, j).
            if i < j {
                extract_edges(bp, i + 1, j, edges);
            }
        }
        Backptr::Matched { k } => {
            edges.push((i, k));
            // Recurse on the inside (i+1..k-1) and the outside (k+1..j).
            if i + 1 < k {
                extract_edges(bp, i + 1, k - 1, edges);
            }
            if k < j {
                extract_edges(bp, k + 1, j, edges);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Injection-relevance check
// ---------------------------------------------------------------------------

/// Check if an edge (global positions) is injection-relevant.
///
/// An edge is injection-relevant if one endpoint has base `Ag` and the other
/// endpoint shares a chunk with a position whose base is `Dir` or `Role`.
fn is_injection_relevant(left: u16, right: u16, types: &[SimpleType], chunk_ids: &[u16]) -> bool {
    let lt = types[left as usize];
    let rt = types[right as usize];

    // One side must be Ag.
    let (ag_pos, other_pos) = if lt.base == TypeId::Ag {
        (left, right)
    } else if rt.base == TypeId::Ag {
        (right, left)
    } else {
        return false;
    };
    let _ = ag_pos; // used for clarity, value not needed further

    // The other side must share a chunk with a Dir or Role.
    let other_chunk = chunk_ids[other_pos as usize];
    for (i, &cid) in chunk_ids.iter().enumerate() {
        if cid == other_chunk {
            let base = types[i].base;
            if base == TypeId::Dir || base == TypeId::Role {
                return true;
            }
        }
    }
    false
}

// ---------------------------------------------------------------------------
// parse()
// ---------------------------------------------------------------------------

/// Parse a sequence of type assignments into a linkage graph.
///
/// The parser is **infallible**: it always returns a [`LinkageGraph`].
/// - Invalid input → empty graph, `timed_out: false`.
/// - Timeout → best-effort graph, `timed_out: true`.
///
/// # Algorithm
///
/// 1. Validate input (monotonic chunk_idx, non-empty TypeExprs).
/// 2. Flatten into a `SimpleType` sequence.
/// 3. Split at conjunction barriers.
/// 4. **Pass 1**: Run Nussinov DP on each segment (max total contractions).
/// 5. Check if any extracted edge is injection-relevant.
/// 6. **Pass 2** (conditional): If Pass 1 found no injection-relevant edges,
///    re-run with bonus scoring for injection-relevant edges.
/// 7. Assemble and return `LinkageGraph`.
pub fn parse(input: &ParseInput, timeout: Option<Duration>) -> LinkageGraph {
    if !validate(input) {
        return LinkageGraph::empty();
    }
    if input.0.is_empty() {
        return LinkageGraph::empty();
    }

    let deadline = timeout.map(|d| Instant::now() + d);
    let (types, chunk_ids) = flatten(input);
    let n = types.len();

    // Build meta.
    let meta: Vec<NodeMeta> = (0..n)
        .map(|i| NodeMeta {
            chunk_idx: chunk_ids[i],
            simple_type: types[i],
        })
        .collect();

    let segments = find_segments(&types);
    if segments.is_empty() {
        return LinkageGraph {
            meta,
            edges: Vec::new(),
            timed_out: false,
        };
    }

    // --- Pass 1: max total contractions ---
    let mut all_edges: Vec<LinkageEdge> = Vec::new();
    let mut timed_out = false;

    for &(seg_start, seg_end) in &segments {
        let seg = &types[seg_start..seg_end];
        if seg.is_empty() {
            continue;
        }

        let no_bonus = |_i: usize, _k: usize| -> usize { 0 };
        let (_, bp, to) = nussinov_dp(seg, &no_bonus, deadline);
        timed_out |= to;

        let seg_len = seg.len();
        if seg_len > 0 {
            let mut local_edges = Vec::new();
            extract_edges(&bp, 0, seg_len - 1, &mut local_edges);

            // Translate local → global.
            for (li, lk) in local_edges {
                let gi = (seg_start + li) as u16;
                let gk = (seg_start + lk) as u16;
                all_edges.push(LinkageEdge {
                    left: gi.min(gk),
                    right: gi.max(gk),
                });
            }
        }
    }

    // Check injection-relevance of Pass 1 edges.
    let has_injection = all_edges
        .iter()
        .any(|e| is_injection_relevant(e.left, e.right, &types, &chunk_ids));

    // --- Pass 2: injection-aware scoring (if needed) ---
    if !has_injection && !timed_out {
        let mut pass2_edges: Vec<LinkageEdge> = Vec::new();
        let mut pass2_has_injection = false;

        for &(seg_start, seg_end) in &segments {
            let seg = &types[seg_start..seg_end];
            if seg.is_empty() {
                continue;
            }
            let seg_len = seg.len();
            let bonus_val = seg_len; // +n bonus for injection-relevant edges

            let bonus_fn = |i: usize, k: usize| -> usize {
                let gi = (seg_start + i) as u16;
                let gk = (seg_start + k) as u16;
                if is_injection_relevant(gi.min(gk), gi.max(gk), &types, &chunk_ids) {
                    bonus_val
                } else {
                    0
                }
            };

            let (_, bp, to) = nussinov_dp(seg, &bonus_fn, deadline);
            timed_out |= to;

            if seg_len > 0 {
                let mut local_edges = Vec::new();
                extract_edges(&bp, 0, seg_len - 1, &mut local_edges);

                for (li, lk) in local_edges {
                    let gi = (seg_start + li) as u16;
                    let gk = (seg_start + lk) as u16;
                    let edge = LinkageEdge {
                        left: gi.min(gk),
                        right: gi.max(gk),
                    };
                    if is_injection_relevant(edge.left, edge.right, &types, &chunk_ids) {
                        pass2_has_injection = true;
                    }
                    pass2_edges.push(edge);
                }
            }
        }

        // Use Pass 2 result if it found injection-relevant edges.
        if pass2_has_injection {
            all_edges = pass2_edges;
        }
    }

    // Sort edges by left for binary search.
    all_edges.sort_by_key(|e| (e.left, e.right));

    LinkageGraph {
        meta,
        edges: all_edges,
        timed_out,
    }
}

// ---------------------------------------------------------------------------
// Proptest Arbitrary for ParseInput
// ---------------------------------------------------------------------------

#[cfg(test)]
pub(crate) mod arb {
    use super::*;
    use crate::types::arb::{arb_type_expr, arb_voiding_kind};
    use proptest::prelude::*;

    pub fn arb_parse_input() -> impl Strategy<Value = ParseInput> {
        proptest::collection::vec(
            (arb_type_expr(), proptest::option::of(arb_voiding_kind())),
            1..=10,
        )
        .prop_map(|items| {
            let assignments: Vec<TypeAssignment> = items
                .into_iter()
                .enumerate()
                .map(|(i, (type_expr, voiding))| TypeAssignment {
                    chunk_idx: i as u16,
                    type_expr,
                    voiding,
                })
                .collect();
            ParseInput(assignments)
        })
    }
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{TypeExpr, VoidingKind};

    /// Helper: build a ParseInput from a list of (chunk_idx, types, voiding).
    fn make_input(items: Vec<(u16, Vec<SimpleType>, Option<VoidingKind>)>) -> ParseInput {
        ParseInput(
            items
                .into_iter()
                .map(|(idx, types, voiding)| TypeAssignment {
                    chunk_idx: idx,
                    type_expr: TypeExpr::new(types),
                    voiding,
                })
                .collect(),
        )
    }

    fn st(base: TypeId, adj: i8) -> SimpleType {
        SimpleType { base, adjoint: adj }
    }

    // 7.12: [ag^l, ag] → 1 edge (0,1)
    #[test]
    fn test_simple_contraction() {
        let input = make_input(vec![(0, vec![st(TypeId::Ag, -1), st(TypeId::Ag, 0)], None)]);
        let g = parse(&input, None);
        assert_eq!(g.edge_count(), 1);
        assert_eq!(g.edges[0], LinkageEdge { left: 0, right: 1 });
        assert!(g.verify());
    }

    // 7.13: [dir, ag^l, ag] → edge (1,2), pos 0 unlinked
    #[test]
    fn test_dir_agl_ag() {
        let input = make_input(vec![(
            0,
            vec![st(TypeId::Dir, 0), st(TypeId::Ag, -1), st(TypeId::Ag, 0)],
            None,
        )]);
        let g = parse(&input, None);
        assert_eq!(g.edge_count(), 1);
        assert_eq!(g.edges[0], LinkageEdge { left: 1, right: 2 });
        assert!(g.verify());
    }

    // 7.14: [dir, usr, n] → 0 edges
    #[test]
    fn test_no_contractions() {
        let input = make_input(vec![
            (0, vec![st(TypeId::Dir, 0)], None),
            (1, vec![st(TypeId::Usr, 0)], None),
            (2, vec![st(TypeId::N, 0)], None),
        ]);
        let g = parse(&input, None);
        assert_eq!(g.edge_count(), 0);
        assert!(g.verify());
    }

    // 7.15: [a^l, b^l, b, a] → nested edges (0,3) and (1,2)
    #[test]
    fn test_nested_planar() {
        let input = make_input(vec![
            (0, vec![st(TypeId::Ag, -1)], None),
            (1, vec![st(TypeId::Dir, -1)], None),
            (2, vec![st(TypeId::Dir, 0)], None),
            (3, vec![st(TypeId::Ag, 0)], None),
        ]);
        let g = parse(&input, None);
        assert_eq!(g.edge_count(), 2);
        assert!(g.edges.contains(&LinkageEdge { left: 0, right: 3 }));
        assert!(g.edges.contains(&LinkageEdge { left: 1, right: 2 }));
        assert!(g.verify());
    }

    // 7.16: empty input → empty graph
    #[test]
    fn test_empty_input() {
        let input = ParseInput(vec![]);
        let g = parse(&input, None);
        assert_eq!(g.meta.len(), 0);
        assert_eq!(g.edge_count(), 0);
        assert!(!g.timed_out);
    }

    // 7.17: conjunction barrier — [ag^l, conj, ag] → 0 edges
    #[test]
    fn test_conj_barrier() {
        let input = make_input(vec![
            (0, vec![st(TypeId::Ag, -1)], None),
            (1, vec![st(TypeId::Conj, 0)], None),
            (2, vec![st(TypeId::Ag, 0)], None),
        ]);
        let g = parse(&input, None);
        assert_eq!(g.edge_count(), 0);
        assert!(g.verify());
    }

    // 7.18: conjunction segments — [ag^l, ag, conj, usr^l, usr] → edges (0,1) and (3,4)
    #[test]
    fn test_conj_segments() {
        let input = make_input(vec![
            (0, vec![st(TypeId::Ag, -1), st(TypeId::Ag, 0)], None),
            (1, vec![st(TypeId::Conj, 0)], None),
            (2, vec![st(TypeId::Usr, -1), st(TypeId::Usr, 0)], None),
        ]);
        let g = parse(&input, None);
        assert_eq!(g.edge_count(), 2);
        assert!(g.edges.contains(&LinkageEdge { left: 0, right: 1 }));
        assert!(g.edges.contains(&LinkageEdge { left: 3, right: 4 }));
        assert!(g.verify());
    }

    // 7.19: intra-chunk self-contraction — [dir, ag^l, ag] single chunk → edge (1,2)
    #[test]
    fn test_intra_chunk_self_contraction() {
        let input = make_input(vec![(
            0,
            vec![st(TypeId::Dir, 0), st(TypeId::Ag, -1), st(TypeId::Ag, 0)],
            None,
        )]);
        let g = parse(&input, None);
        assert_eq!(g.edge_count(), 1);
        assert_eq!(g.edges[0], LinkageEdge { left: 1, right: 2 });
        assert!(g.verify());
    }

    // 7.20: security Pass 2 — ambiguous parse where max-contraction hides injection
    //
    // [n^l, dir, ag^l, n, ag]  (positions 0..4, chunks 0,1,1,2,3)
    // Option A (Pass 1): (0,3) n^l↔n = 1 edge, no injection. Crosses with (2,4).
    // Option B (Pass 2): (2,4) ag^l↔ag = 1 edge, injection-relevant (pos 2 shares chunk 1 with Dir at pos 1).
    // Pass 1 picks (0,3) first (k=3 found before k=4 for i=0). No injection → triggers Pass 2.
    // Pass 2 bonus makes (2,4) score higher → returned.
    #[test]
    fn test_pass2_surfaces_injection() {
        let input = make_input(vec![
            (0, vec![st(TypeId::N, -1)], None), // pos 0: n^l
            (1, vec![st(TypeId::Dir, 0), st(TypeId::Ag, -1)], None), // pos 1: dir, pos 2: ag^l
            (2, vec![st(TypeId::N, 0)], None),  // pos 3: n
            (3, vec![st(TypeId::Ag, 0)], None), // pos 4: ag
        ]);
        let g = parse(&input, None);
        let has_ag_edge = g.edges.iter().any(|e| {
            let lb = g.meta[e.left as usize].simple_type.base;
            let rb = g.meta[e.right as usize].simple_type.base;
            lb == TypeId::Ag && rb == TypeId::Ag
        });
        assert!(
            has_ag_edge,
            "Pass 2 should surface the injection-relevant ag^l↔ag edge.\nGraph: {g}"
        );
        assert!(g.verify());
    }

    // 7.21: Pass 2 fallback — no injection anywhere, Pass 2 returns same as Pass 1
    #[test]
    fn test_pass2_fallback_same_as_pass1() {
        let input = make_input(vec![
            (0, vec![st(TypeId::N, -1), st(TypeId::N, 0)], None),
            (1, vec![st(TypeId::S, -1), st(TypeId::S, 0)], None),
        ]);
        let g = parse(&input, None);
        #[allow(unused_variables)]
        let _ = &g; // Pass 2 runs but finds nothing injection-relevant.
                    // Both are non-injection edges. Pass 2 runs but finds nothing injection-relevant.
                    // Should return same edges as Pass 1.
        assert_eq!(g.edge_count(), 2);
        assert!(g.edges.contains(&LinkageEdge { left: 0, right: 1 }));
        assert!(g.edges.contains(&LinkageEdge { left: 2, right: 3 }));
        assert!(g.verify());
    }

    // 7.22: multi-element conj chunk
    #[test]
    fn test_multi_element_conj_chunk() {
        // chunk 0: [ag^l, ag], chunk 1: [conj, n^r], chunk 2: [usr^l, usr]
        // positions: 0:ag^l 1:ag 2:conj 3:n^r 4:usr^l 5:usr
        // Segment 1: [0,1] (ag^l, ag). Segment 2: [3,4,5] (n^r, usr^l, usr).
        // conj at pos 2 excluded. n^r (pos 3) joins segment 2.
        let input = make_input(vec![
            (0, vec![st(TypeId::Ag, -1), st(TypeId::Ag, 0)], None),
            (1, vec![st(TypeId::Conj, 0), st(TypeId::N, 1)], None),
            (2, vec![st(TypeId::Usr, -1), st(TypeId::Usr, 0)], None),
        ]);
        let g = parse(&input, None);
        // Segment 1: ag^l↔ag → edge (0,1)
        // Segment 2: [n^r, usr^l, usr] — n^r doesn't contract with usr^l or usr. usr^l↔usr → edge (4,5)
        assert!(g.edges.contains(&LinkageEdge { left: 0, right: 1 }));
        assert!(g.edges.contains(&LinkageEdge { left: 4, right: 5 }));
        // No edge crosses conj position 2.
        for e in &g.edges {
            assert!(!(e.left < 2 && e.right > 2), "edge crosses conj barrier");
        }
        assert!(g.verify());
    }

    // 7.23: input validation — non-monotonic chunk_idx
    #[test]
    fn test_validation_non_monotonic() {
        let input = ParseInput(vec![
            TypeAssignment {
                chunk_idx: 0,
                type_expr: TypeExpr::new(vec![st(TypeId::Dir, 0)]),
                voiding: None,
            },
            TypeAssignment {
                chunk_idx: 2,
                type_expr: TypeExpr::new(vec![st(TypeId::Ag, 0)]),
                voiding: None,
            },
            TypeAssignment {
                chunk_idx: 1, // non-monotonic!
                type_expr: TypeExpr::new(vec![st(TypeId::Usr, 0)]),
                voiding: None,
            },
        ]);
        let g = parse(&input, None);
        assert_eq!(g.meta.len(), 0);
        assert_eq!(g.edge_count(), 0);
    }

    // 7.24: input validation — empty TypeExpr
    #[test]
    fn test_validation_empty_type_expr() {
        let input = ParseInput(vec![TypeAssignment {
            chunk_idx: 0,
            type_expr: TypeExpr::new(vec![]),
            voiding: None,
        }]);
        let g = parse(&input, None);
        assert_eq!(g.meta.len(), 0);
        assert_eq!(g.edge_count(), 0);
    }
}
