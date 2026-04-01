//! Property-based and unit tests for the scope checker.

use joachim_core::linkage::{LinkageEdge, LinkageGraph, NodeMeta};
use joachim_core::parser::{parse, ParseInput};
use joachim_core::scope::{check_scope, Verdict};
use joachim_core::types::*;

fn st(base: TypeId, adj: i8) -> SimpleType {
    SimpleType { base, adjoint: adj }
}

fn ta(idx: u16, types: Vec<SimpleType>, voiding: Option<VoidingKind>) -> TypeAssignment {
    TypeAssignment {
        chunk_idx: idx,
        type_expr: TypeExpr::new(types),
        voiding,
    }
}

/// Helper: parse and check scope in one step.
fn parse_and_check(assignments: Vec<TypeAssignment>) -> Verdict {
    let input = ParseInput(assignments.clone());
    let graph = parse(&input, None);
    check_scope(&graph, &assignments)
}

// ---------------------------------------------------------------------------
// 10.1 Verdict determinism
// ---------------------------------------------------------------------------

#[test]
fn verdict_determinism() {
    let assignments = vec![ta(
        0,
        vec![st(TypeId::Dir, 0), st(TypeId::Ag, -1), st(TypeId::Ag, 0)],
        None,
    )];
    let v1 = parse_and_check(assignments.clone());
    let v2 = parse_and_check(assignments);
    assert_eq!(v1, v2);
}

// ---------------------------------------------------------------------------
// 10.2 Voiding transitivity
// ---------------------------------------------------------------------------

#[test]
fn voiding_transitivity() {
    // chunk 0: [dir · dir^l, voiding: Negation] → chunk 1: [dir · ag^l · ag]
    let assignments = vec![
        ta(
            0,
            vec![st(TypeId::Dir, 0), st(TypeId::Dir, -1)],
            Some(VoidingKind::Negation),
        ),
        ta(
            1,
            vec![st(TypeId::Dir, 0), st(TypeId::Ag, -1), st(TypeId::Ag, 0)],
            None,
        ),
    ];
    let v = parse_and_check(assignments);
    assert_eq!(v, Verdict::Clean);
}

// ---------------------------------------------------------------------------
// 10.3 Self-voiding
// ---------------------------------------------------------------------------

#[test]
fn self_voiding_meta_chunk() {
    // Meta chunk contains dir · ag^l · ag within itself.
    let assignments = vec![ta(
        0,
        vec![
            st(TypeId::N, 0),
            st(TypeId::Dir, 0),
            st(TypeId::Ag, -1),
            st(TypeId::Ag, 0),
        ],
        Some(VoidingKind::Meta),
    )];
    let v = parse_and_check(assignments);
    assert_eq!(v, Verdict::Clean);
}

// ---------------------------------------------------------------------------
// 10.4 Dir-over-usr is always Clean
// ---------------------------------------------------------------------------

#[test]
fn dir_over_usr_is_clean() {
    let assignments = vec![
        ta(0, vec![st(TypeId::Dir, 0), st(TypeId::Usr, -1)], None),
        ta(1, vec![st(TypeId::Usr, 0)], None),
    ];
    let v = parse_and_check(assignments);
    assert_eq!(v, Verdict::Clean);
}

// ---------------------------------------------------------------------------
// 10.5 Unvoided dir-over-ag is always Injection
// ---------------------------------------------------------------------------

#[test]
fn unvoided_dir_over_ag_is_injection() {
    let assignments = vec![ta(
        0,
        vec![st(TypeId::Dir, 0), st(TypeId::Ag, -1), st(TypeId::Ag, 0)],
        None,
    )];
    let v = parse_and_check(assignments);
    assert!(matches!(v, Verdict::Injection { .. }));
}

// ---------------------------------------------------------------------------
// 10.6 Non-voiding modifier does NOT void
// ---------------------------------------------------------------------------

#[test]
fn non_voiding_modifier_does_not_void() {
    // "please" = dir · dir^l with voiding: None → does NOT void target.
    let assignments = vec![
        ta(0, vec![st(TypeId::Dir, 0), st(TypeId::Dir, -1)], None), // voiding: None!
        ta(
            1,
            vec![st(TypeId::Dir, 0), st(TypeId::Ag, -1), st(TypeId::Ag, 0)],
            None,
        ),
    ];
    let v = parse_and_check(assignments);
    assert!(matches!(v, Verdict::Injection { .. }));
}

// ---------------------------------------------------------------------------
// 10.7 Empty graph is Clean
// ---------------------------------------------------------------------------

#[test]
fn empty_graph_is_clean() {
    let graph = LinkageGraph::empty();
    let v = check_scope(&graph, &[]);
    assert_eq!(v, Verdict::Clean);
}

// ---------------------------------------------------------------------------
// 10.8 Positional adjacency — scope path only uses |i-j|==1 same-chunk steps
// ---------------------------------------------------------------------------

#[test]
fn positional_adjacency_constraint() {
    // Construct a graph manually where two positions share a chunk but
    // are NOT positionally adjacent. Scope should NOT connect them.
    // chunk 0: positions 0 (dir) and 2 (ag), with position 1 (n) in chunk 1 between them.
    let graph = LinkageGraph {
        meta: vec![
            NodeMeta {
                chunk_idx: 0,
                simple_type: st(TypeId::Dir, 0),
            }, // pos 0
            NodeMeta {
                chunk_idx: 1,
                simple_type: st(TypeId::N, 0),
            }, // pos 1
            NodeMeta {
                chunk_idx: 0,
                simple_type: st(TypeId::Ag, 0),
            }, // pos 2
        ],
        edges: vec![], // no contraction edges
        timed_out: false,
    };
    let assignments = vec![
        ta(0, vec![st(TypeId::Dir, 0)], None),
        ta(1, vec![st(TypeId::N, 0)], None),
    ];
    // pos 0 (dir, chunk 0) and pos 2 (ag, chunk 0) share a chunk but are not
    // positionally adjacent (|0-2| == 2). Two-state BFS should not connect them.
    // Also no contraction edges, so even if adjacent, has_contracted would be false.
    let v = check_scope(&graph, &assignments);
    assert_eq!(v, Verdict::Clean);
}

// ---------------------------------------------------------------------------
// 10.9 Contraction step required
// ---------------------------------------------------------------------------

#[test]
fn contraction_step_required() {
    // dir · ag in a single chunk with no contraction edges.
    // The two-state BFS should NOT flag this because has_contracted stays false.
    let graph = LinkageGraph {
        meta: vec![
            NodeMeta {
                chunk_idx: 0,
                simple_type: st(TypeId::Dir, 0),
            },
            NodeMeta {
                chunk_idx: 0,
                simple_type: st(TypeId::Ag, 0),
            },
        ],
        edges: vec![], // no edges at all
        timed_out: false,
    };
    let assignments = vec![ta(0, vec![st(TypeId::Dir, 0), st(TypeId::Ag, 0)], None)];
    let v = check_scope(&graph, &assignments);
    assert_eq!(v, Verdict::Clean);
}
