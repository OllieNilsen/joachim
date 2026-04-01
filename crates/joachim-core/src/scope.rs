//! Scope checker: detects `dir → ag` and `role → ag` injection patterns.
//!
//! Analyzes a [`LinkageGraph`] via two-state BFS traversal following
//! positionally-adjacent same-chunk steps and contraction edges. Requires at
//! least one contraction step per scope path. Voiding is chunk-granular.

use std::collections::{HashSet, VecDeque};

use smallvec::SmallVec;

use crate::linkage::LinkageGraph;
use crate::types::{TypeAssignment, TypeId};

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

/// The kind of injection scope pattern detected.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum ScopePattern {
    /// A directive (`dir`) scopes over an agent-domain action (`ag`).
    DirOverAg,
    /// A role assignment (`role`) scopes over an agent-domain action (`ag`).
    RoleOverAg,
}

/// A single scope violation found in the linkage graph.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScopeViolation {
    /// The pattern detected.
    pub pattern: ScopePattern,
    /// Flattened-sequence position of the `dir` or `role` source.
    pub source_pos: u16,
    /// Flattened-sequence position of the `ag` target.
    pub target_pos: u16,
}

/// The verdict of the scope checker.
#[non_exhaustive]
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Verdict {
    /// At least one unvoided injection pattern was found.
    Injection {
        /// All unvoided scope violations.
        violations: Vec<ScopeViolation>,
    },
    /// No unvoided injection patterns.
    Clean,
}

// ---------------------------------------------------------------------------
// EdgeKind — tags adjacency list entries
// ---------------------------------------------------------------------------

/// Distinguishes contraction edges from same-chunk adjacency edges in the
/// combined adjacency list used by the two-state BFS.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum EdgeKind {
    /// A contraction edge from the linkage graph.
    Contraction,
    /// A positionally-adjacent same-chunk step (`|i-j| == 1`, same `chunk_idx`).
    Adjacency,
}

// ---------------------------------------------------------------------------
// build_adjacency
// ---------------------------------------------------------------------------

/// Build a bidirectional adjacency list combining contraction edges and
/// positionally-adjacent same-chunk pairs.
pub fn build_adjacency(graph: &LinkageGraph) -> Vec<SmallVec<[(u16, EdgeKind); 4]>> {
    let n = graph.meta.len();
    let mut adj: Vec<SmallVec<[(u16, EdgeKind); 4]>> = vec![SmallVec::new(); n];

    // Contraction edges (bidirectional).
    for e in &graph.edges {
        adj[e.left as usize].push((e.right, EdgeKind::Contraction));
        adj[e.right as usize].push((e.left, EdgeKind::Contraction));
    }

    // Adjacent same-chunk pairs.
    for (i, pair) in graph.meta.windows(2).enumerate() {
        if pair[0].chunk_idx == pair[1].chunk_idx {
            let left = i as u16;
            let right = (i + 1) as u16;
            adj[i].push((right, EdgeKind::Adjacency));
            adj[i + 1].push((left, EdgeKind::Adjacency));
        }
    }

    adj
}

// ---------------------------------------------------------------------------
// compute_voided_chunks
// ---------------------------------------------------------------------------

/// Compute the set of voided chunk indices via BFS.
///
/// Seeds: all chunks with `voiding: Some(_)`.
/// Expansion: contraction edges crossing into non-voided chunks.
pub fn compute_voided_chunks(graph: &LinkageGraph, assignments: &[TypeAssignment]) -> HashSet<u16> {
    let mut voided: HashSet<u16> = HashSet::new();
    let mut queue: VecDeque<u16> = VecDeque::new();

    // Seed with self-voiding chunks.
    for ta in assignments {
        if ta.voiding.is_some() && voided.insert(ta.chunk_idx) {
            queue.push_back(ta.chunk_idx);
        }
    }

    // BFS over chunks via contraction edges.
    while let Some(chunk) = queue.pop_front() {
        for e in &graph.edges {
            let left_chunk = graph.meta[e.left as usize].chunk_idx;
            let right_chunk = graph.meta[e.right as usize].chunk_idx;

            let target = match (left_chunk == chunk, right_chunk == chunk) {
                (true, false) => right_chunk,
                (false, true) => left_chunk,
                _ => continue, // both same chunk or neither
            };

            if voided.insert(target) {
                queue.push_back(target);
            }
        }
    }

    voided
}

// ---------------------------------------------------------------------------
// find_scope_paths
// ---------------------------------------------------------------------------

/// Find all scope paths (`dir → ag` and `role → ag`) using two-state BFS.
///
/// State: `(position, has_contracted)`. An `Ag` node is only reported when
/// `has_contracted == true`. This prevents pure same-chunk adjacency paths.
pub fn find_scope_paths(
    graph: &LinkageGraph,
    adjacency: &[SmallVec<[(u16, EdgeKind); 4]>],
) -> Vec<(u16, u16, ScopePattern)> {
    let n = graph.meta.len();
    let mut results = Vec::new();

    for start in 0..n {
        let base = graph.meta[start].simple_type.base;
        let pattern = match base {
            TypeId::Dir => ScopePattern::DirOverAg,
            TypeId::Role => ScopePattern::RoleOverAg,
            _ => continue,
        };

        // Two-state BFS: visited[pos][has_contracted].
        let mut visited = vec![[false; 2]; n];
        let mut queue: VecDeque<(u16, bool)> = VecDeque::new();

        visited[start][0] = true; // state: (start, has_contracted=false)
        queue.push_back((start as u16, false));

        while let Some((pos, has_contracted)) = queue.pop_front() {
            for &(neighbor, kind) in &adjacency[pos as usize] {
                let new_contracted = has_contracted || kind == EdgeKind::Contraction;
                let state_idx = usize::from(new_contracted);

                if !visited[neighbor as usize][state_idx] {
                    visited[neighbor as usize][state_idx] = true;
                    queue.push_back((neighbor, new_contracted));

                    // Report if we reached an Ag node with at least one contraction.
                    if new_contracted
                        && graph.meta[neighbor as usize].simple_type.base == TypeId::Ag
                    {
                        results.push((start as u16, neighbor, pattern));
                    }
                }
            }
        }
    }

    results
}

// ---------------------------------------------------------------------------
// check_scope
// ---------------------------------------------------------------------------

/// Check a linkage graph for injection patterns.
///
/// Returns [`Verdict::Injection`] if any unvoided `dir → ag` or `role → ag`
/// path exists, [`Verdict::Clean`] otherwise.
pub fn check_scope(graph: &LinkageGraph, assignments: &[TypeAssignment]) -> Verdict {
    if graph.meta.is_empty() || graph.edges.is_empty() {
        return Verdict::Clean;
    }

    let adjacency = build_adjacency(graph);
    let voided = compute_voided_chunks(graph, assignments);
    let paths = find_scope_paths(graph, &adjacency);

    let violations: Vec<ScopeViolation> = paths
        .into_iter()
        .filter(|&(src, tgt, _)| {
            let src_chunk = graph.meta[src as usize].chunk_idx;
            let tgt_chunk = graph.meta[tgt as usize].chunk_idx;
            !voided.contains(&src_chunk) && !voided.contains(&tgt_chunk)
        })
        .map(|(src, tgt, pattern)| ScopeViolation {
            pattern,
            source_pos: src,
            target_pos: tgt,
        })
        .collect();

    if violations.is_empty() {
        Verdict::Clean
    } else {
        Verdict::Injection { violations }
    }
}
