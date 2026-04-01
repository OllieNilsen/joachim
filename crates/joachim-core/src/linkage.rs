//! Linkage graph: the parser output.
//!
//! A [`LinkageGraph`] represents the maximal planar linkage found by the
//! Nussinov parser. Node identity is the vector index into [`meta`](LinkageGraph::meta).
//! Edges are non-crossing contraction pairs, sorted by [`left`](LinkageEdge::left).

use crate::types::{can_contract, SimpleType};
use std::fmt;

// ---------------------------------------------------------------------------
// NodeMeta
// ---------------------------------------------------------------------------

/// Metadata for one position in the flattened `SimpleType` sequence.
#[derive(Copy, Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct NodeMeta {
    /// Which chunk this position came from.
    pub chunk_idx: u16,
    /// The simple type at this position.
    pub simple_type: SimpleType,
}

// ---------------------------------------------------------------------------
// LinkageEdge
// ---------------------------------------------------------------------------

/// A contraction edge connecting two positions in the flattened sequence.
///
/// Invariant: `left < right`.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct LinkageEdge {
    /// Index of the left position (lower).
    pub left: u16,
    /// Index of the right position (higher).
    pub right: u16,
}

// ---------------------------------------------------------------------------
// LinkageGraph
// ---------------------------------------------------------------------------

/// The complete linkage graph produced by the parser.
///
/// Nodes are implicit — `meta[i]` describes position `i`. Edges are the
/// non-crossing contraction edges, sorted by `left` for binary search.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct LinkageGraph {
    /// One entry per position in the flattened sequence.
    pub meta: Vec<NodeMeta>,
    /// Non-crossing contraction edges, sorted by `left`.
    pub edges: Vec<LinkageEdge>,
    /// Whether the parse timed out before completing.
    pub timed_out: bool,
}

impl LinkageGraph {
    /// Create an empty graph (no nodes, no edges, not timed out).
    #[must_use]
    pub fn empty() -> Self {
        Self {
            meta: Vec::new(),
            edges: Vec::new(),
            timed_out: false,
        }
    }

    /// Number of contraction edges.
    #[must_use]
    pub fn edge_count(&self) -> usize {
        self.edges.len()
    }

    /// Iterate over edges where `left == pos`.
    ///
    /// Uses binary search on the sorted edge vec. O(log E + k) where k is
    /// the number of matching edges. Finding edges where `right == pos`
    /// requires an O(E) scan — use [`build_adjacency`](crate::scope::build_adjacency)
    /// for bidirectional lookup.
    pub fn edges_from(&self, pos: u16) -> impl Iterator<Item = &LinkageEdge> {
        // Find the first edge with left >= pos via binary search.
        let start = self.edges.partition_point(|e| e.left < pos);
        self.edges[start..]
            .iter()
            .take_while(move |e| e.left == pos)
    }

    /// Verify structural invariants: valid contractions, non-crossing, in bounds.
    ///
    /// Intended for testing/debugging, not hot-path use.
    #[must_use]
    pub fn verify(&self) -> bool {
        let n = self.meta.len() as u16;

        for e in &self.edges {
            // Bounds check.
            if e.left >= n || e.right >= n {
                return false;
            }
            // left < right.
            if e.left >= e.right {
                return false;
            }
            // Valid contraction.
            if !can_contract(
                self.meta[e.left as usize].simple_type,
                self.meta[e.right as usize].simple_type,
            ) {
                return false;
            }
        }

        // Non-crossing: for sorted edges, check pairwise.
        for window in self.edges.windows(2) {
            let a = &window[0];
            let b = &window[1];
            // Since sorted by left: a.left <= b.left.
            // Crossing: a.left < b.left < a.right < b.right.
            if a.left < b.left && b.left < a.right && a.right < b.right {
                return false;
            }
        }

        true
    }
}

impl fmt::Display for LinkageGraph {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(
            f,
            "LinkageGraph ({} nodes, {} edges{})",
            self.meta.len(),
            self.edges.len(),
            if self.timed_out { ", TIMED OUT" } else { "" },
        )?;
        for (i, m) in self.meta.iter().enumerate() {
            writeln!(f, "  [{i}] chunk={} type={}", m.chunk_idx, m.simple_type)?;
        }
        for e in &self.edges {
            writeln!(
                f,
                "  edge ({}, {}): {} ↔ {}",
                e.left,
                e.right,
                self.meta[e.left as usize].simple_type,
                self.meta[e.right as usize].simple_type,
            )?;
        }
        Ok(())
    }
}
