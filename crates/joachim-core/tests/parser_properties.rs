//! Property-based tests for the Nussinov parser.

use joachim_core::linkage::LinkageEdge;
use joachim_core::parser::{parse, ParseInput};
use joachim_core::types::*;
use proptest::prelude::*;

// ---------------------------------------------------------------------------
// Helpers: rebuild strategies for integration tests.
// ---------------------------------------------------------------------------

fn arb_type_id() -> impl Strategy<Value = TypeId> {
    prop_oneof![
        Just(TypeId::Dir),
        Just(TypeId::Ag),
        Just(TypeId::Usr),
        Just(TypeId::Role),
        Just(TypeId::S),
        Just(TypeId::N),
        Just(TypeId::Conj),
        Just(TypeId::Ass),
        Just(TypeId::Qst),
    ]
}

fn arb_simple_type() -> impl Strategy<Value = SimpleType> {
    (arb_type_id(), -3i8..=3i8).prop_map(|(base, adjoint)| SimpleType { base, adjoint })
}

fn arb_type_expr() -> impl Strategy<Value = TypeExpr> {
    proptest::collection::vec(arb_simple_type(), 1..=5).prop_map(TypeExpr::new)
}

fn arb_voiding_kind() -> impl Strategy<Value = VoidingKind> {
    prop_oneof![
        Just(VoidingKind::Hypothetical),
        Just(VoidingKind::Negation),
        Just(VoidingKind::Meta),
    ]
}

fn arb_parse_input() -> impl Strategy<Value = ParseInput> {
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

fn st(base: TypeId, adj: i8) -> SimpleType {
    SimpleType { base, adjoint: adj }
}

// ---------------------------------------------------------------------------
// 8.1 Parse determinism
// ---------------------------------------------------------------------------

proptest! {
    #[test]
    fn parse_determinism(input in arb_parse_input()) {
        let g1 = parse(&input, None);
        let g2 = parse(&input, None);
        prop_assert_eq!(g1.edges, g2.edges);
        prop_assert_eq!(g1.meta.len(), g2.meta.len());
    }
}

// ---------------------------------------------------------------------------
// 8.2 Termination
// ---------------------------------------------------------------------------

proptest! {
    #[test]
    fn parse_terminates(input in arb_parse_input()) {
        let _g = parse(&input, None);
        // If we get here, it terminated.
    }
}

// ---------------------------------------------------------------------------
// 8.3 Edge validity
// ---------------------------------------------------------------------------

proptest! {
    #[test]
    fn edge_validity(input in arb_parse_input()) {
        let g = parse(&input, None);
        for e in &g.edges {
            let left_type = g.meta[e.left as usize].simple_type;
            let right_type = g.meta[e.right as usize].simple_type;
            prop_assert!(
                can_contract(left_type, right_type),
                "Invalid contraction: {} ({}) ↔ {} ({})",
                left_type, e.left, right_type, e.right,
            );
        }
    }
}

// ---------------------------------------------------------------------------
// 8.4 Planarity
// ---------------------------------------------------------------------------

proptest! {
    #[test]
    fn planarity(input in arb_parse_input()) {
        let g = parse(&input, None);
        for i in 0..g.edges.len() {
            for j in (i + 1)..g.edges.len() {
                let a = &g.edges[i];
                let b = &g.edges[j];
                // Crossing: a.left < b.left < a.right < b.right
                let crosses = a.left < b.left && b.left < a.right && a.right < b.right;
                prop_assert!(!crosses, "Crossing edges: ({},{}) and ({},{})", a.left, a.right, b.left, b.right);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// 8.5 Edge count bound
// ---------------------------------------------------------------------------

proptest! {
    #[test]
    fn edge_count_bound(input in arb_parse_input()) {
        let g = parse(&input, None);
        let n = g.meta.len();
        prop_assert!(g.edge_count() <= n / 2, "Too many edges: {} > {}/2", g.edge_count(), n);
    }
}

// ---------------------------------------------------------------------------
// 8.6 Adjoint pairs always contract
// ---------------------------------------------------------------------------

proptest! {
    #[test]
    fn adjoint_pairs_contract(base in arb_type_id()) {
        prop_assume!(base != TypeId::Conj);
        let input = ParseInput(vec![
            TypeAssignment {
                chunk_idx: 0,
                type_expr: TypeExpr::new(vec![
                    SimpleType { base, adjoint: -1 },
                    SimpleType { base, adjoint: 0 },
                ]),
                voiding: None,
            },
        ]);
        let g = parse(&input, None);
        prop_assert_eq!(g.edge_count(), 1, "Adjoint pair should produce exactly 1 edge");
        prop_assert_eq!(g.edges[0], LinkageEdge { left: 0, right: 1 });
    }
}

// ---------------------------------------------------------------------------
// 8.7 No contractions for identical primitives
// ---------------------------------------------------------------------------

proptest! {
    #[test]
    fn identical_primitives_no_contract(base in arb_type_id()) {
        let input = ParseInput(vec![
            TypeAssignment {
                chunk_idx: 0,
                type_expr: TypeExpr::new(vec![
                    SimpleType { base, adjoint: 0 },
                    SimpleType { base, adjoint: 0 },
                    SimpleType { base, adjoint: 0 },
                ]),
                voiding: None,
            },
        ]);
        let g = parse(&input, None);
        prop_assert_eq!(g.edge_count(), 0, "Identical primitives should not contract");
    }
}

// ---------------------------------------------------------------------------
// 8.8 Conjunction barrier
// ---------------------------------------------------------------------------

proptest! {
    #[test]
    fn conjunction_barrier(input in arb_parse_input()) {
        let g = parse(&input, None);
        // Find all conj positions.
        let conj_positions: Vec<u16> = g.meta.iter().enumerate()
            .filter(|(_, m)| m.simple_type.base == TypeId::Conj)
            .map(|(i, _)| i as u16)
            .collect();
        // No edge may cross a conj position.
        for e in &g.edges {
            for &cp in &conj_positions {
                prop_assert!(
                    !(e.left < cp && cp < e.right),
                    "Edge ({},{}) crosses conj at position {}", e.left, e.right, cp,
                );
            }
        }
    }
}

// ---------------------------------------------------------------------------
// 8.9 Nested pairs
// ---------------------------------------------------------------------------

proptest! {
    #[test]
    fn nested_pairs(
        outer_base in arb_type_id(),
        inner_base in arb_type_id(),
    ) {
        prop_assume!(outer_base != TypeId::Conj && inner_base != TypeId::Conj);
        let input = ParseInput(vec![
            TypeAssignment {
                chunk_idx: 0,
                type_expr: TypeExpr::new(vec![st(outer_base, -1)]),
                voiding: None,
            },
            TypeAssignment {
                chunk_idx: 1,
                type_expr: TypeExpr::new(vec![st(inner_base, -1)]),
                voiding: None,
            },
            TypeAssignment {
                chunk_idx: 2,
                type_expr: TypeExpr::new(vec![st(inner_base, 0)]),
                voiding: None,
            },
            TypeAssignment {
                chunk_idx: 3,
                type_expr: TypeExpr::new(vec![st(outer_base, 0)]),
                voiding: None,
            },
        ]);
        let g = parse(&input, None);
        prop_assert_eq!(g.edge_count(), 2, "Nested pairs should produce 2 edges");
        let outer = LinkageEdge { left: 0, right: 3 };
        let inner = LinkageEdge { left: 1, right: 2 };
        prop_assert!(g.edges.contains(&outer), "Missing outer edge (0,3)");
        prop_assert!(g.edges.contains(&inner), "Missing inner edge (1,2)");
    }
}

// ---------------------------------------------------------------------------
// 8.10 All edges use global positions
// ---------------------------------------------------------------------------

proptest! {
    #[test]
    fn edges_use_global_positions(input in arb_parse_input()) {
        let g = parse(&input, None);
        let n = g.meta.len() as u16;
        for e in &g.edges {
            prop_assert!(e.left < n, "Edge left {} >= meta.len() {}", e.left, n);
            prop_assert!(e.right < n, "Edge right {} >= meta.len() {}", e.right, n);
        }
    }
}
