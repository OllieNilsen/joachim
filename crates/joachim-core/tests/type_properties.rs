//! Property-based tests for the pregroup type algebra.

use joachim_core::types::*;
use proptest::prelude::*;

// ---------------------------------------------------------------------------
// Helpers: reuse the crate's internal arb strategies via a local mirror.
// (The arb module is pub(crate), so we rebuild equivalent strategies here.)
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

// ---------------------------------------------------------------------------
// 5.1 Left contraction
// ---------------------------------------------------------------------------

proptest! {
    #[test]
    fn left_contraction(a in arb_simple_type()) {
        prop_assert!(can_contract(a.left_adj(), a));
    }
}

// ---------------------------------------------------------------------------
// 5.2 Right contraction
// ---------------------------------------------------------------------------

proptest! {
    #[test]
    fn right_contraction(a in arb_simple_type()) {
        prop_assert!(can_contract(a, a.right_adj()));
    }
}

// ---------------------------------------------------------------------------
// 5.3 Contraction formula consistency
// ---------------------------------------------------------------------------

proptest! {
    #[test]
    fn contraction_formula_consistency(
        x in arb_simple_type(),
        y in arb_simple_type(),
    ) {
        let expected = x.base == y.base
            && y.adjoint.checked_sub(1).map_or(false, |r| x.adjoint == r);
        prop_assert_eq!(can_contract(x, y), expected);
    }
}

// ---------------------------------------------------------------------------
// 5.4 Mixed adjoint cancellation
// ---------------------------------------------------------------------------

proptest! {
    #[test]
    fn left_right_cancellation(a in arb_simple_type()) {
        prop_assert_eq!(a.left_adj().right_adj(), a);
    }

    #[test]
    fn right_left_cancellation(a in arb_simple_type()) {
        prop_assert_eq!(a.right_adj().left_adj(), a);
    }
}

// ---------------------------------------------------------------------------
// 5.5 Double left adjoint is distinct
// ---------------------------------------------------------------------------

proptest! {
    #[test]
    fn double_left_adj_distinct(a in arb_simple_type()) {
        prop_assert_ne!(a.left_adj().left_adj(), a);
    }
}

// ---------------------------------------------------------------------------
// 5.6 Double right adjoint is distinct
// ---------------------------------------------------------------------------

proptest! {
    #[test]
    fn double_right_adj_distinct(a in arb_simple_type()) {
        prop_assert_ne!(a.right_adj().right_adj(), a);
    }
}

// ---------------------------------------------------------------------------
// 5.7 Mismatched base types don't contract
// ---------------------------------------------------------------------------

proptest! {
    #[test]
    fn mismatched_base_no_contract(
        base_a in arb_type_id(),
        base_b in arb_type_id(),
        adj_a in -3i8..=3i8,
        adj_b in -3i8..=3i8,
    ) {
        prop_assume!(base_a != base_b);
        let a = SimpleType { base: base_a, adjoint: adj_a };
        let b = SimpleType { base: base_b, adjoint: adj_b };
        prop_assert!(!can_contract(a, b));
    }
}

// ---------------------------------------------------------------------------
// 5.8 Mismatched adjoint gap doesn't contract
// ---------------------------------------------------------------------------

proptest! {
    #[test]
    fn mismatched_adjoint_no_contract(
        base in arb_type_id(),
        adj_a in -3i8..=3i8,
        adj_b in -3i8..=3i8,
    ) {
        prop_assume!(adj_a != adj_b.wrapping_sub(1));
        let a = SimpleType { base, adjoint: adj_a };
        let b = SimpleType { base, adjoint: adj_b };
        prop_assert!(!can_contract(a, b));
    }
}

// ---------------------------------------------------------------------------
// 5.9 TypeExpr concat associativity
// ---------------------------------------------------------------------------

proptest! {
    #[test]
    fn type_expr_concat_assoc(
        a in arb_type_expr(),
        b in arb_type_expr(),
        c in arb_type_expr(),
    ) {
        let ab_c = a.clone().concat(b.clone()).concat(c.clone());
        let a_bc = a.concat(b.concat(c));
        prop_assert_eq!(ab_c, a_bc);
    }
}

// ---------------------------------------------------------------------------
// 5.10 TypeExpr unit identity
// ---------------------------------------------------------------------------

proptest! {
    #[test]
    fn type_expr_left_identity(a in arb_type_expr()) {
        prop_assert_eq!(TypeExpr::unit().concat(a.clone()), a);
    }

    #[test]
    fn type_expr_right_identity(a in arb_type_expr()) {
        prop_assert_eq!(a.clone().concat(TypeExpr::unit()), a);
    }
}

// ---------------------------------------------------------------------------
// 5.11 TypeExpr left adjoint is contravariant
// ---------------------------------------------------------------------------

proptest! {
    #[test]
    fn type_expr_left_adj_contravariant(e in arb_type_expr()) {
        let result = e.left_adj();
        let expected: Vec<SimpleType> = e.as_slice().iter().rev().map(|t| t.left_adj()).collect();
        prop_assert_eq!(result.as_slice(), expected.as_slice());
    }
}

// ---------------------------------------------------------------------------
// 5.12 TypeExpr right adjoint is contravariant
// ---------------------------------------------------------------------------

proptest! {
    #[test]
    fn type_expr_right_adj_contravariant(e in arb_type_expr()) {
        let result = e.right_adj();
        let expected: Vec<SimpleType> = e.as_slice().iter().rev().map(|t| t.right_adj()).collect();
        prop_assert_eq!(result.as_slice(), expected.as_slice());
    }
}

// ---------------------------------------------------------------------------
// 5.13 TypeExpr adjoint involution
// ---------------------------------------------------------------------------

proptest! {
    #[test]
    fn type_expr_left_right_involution(e in arb_type_expr()) {
        prop_assert_eq!(e.left_adj().right_adj(), e);
    }

    #[test]
    fn type_expr_right_left_involution(e in arb_type_expr()) {
        prop_assert_eq!(e.right_adj().left_adj(), e);
    }
}

// ---------------------------------------------------------------------------
// 5.14 SimpleType equality is reflexive, symmetric, transitive
// ---------------------------------------------------------------------------

proptest! {
    #[test]
    fn simple_type_eq_reflexive(a in arb_simple_type()) {
        prop_assert_eq!(a, a);
    }

    #[test]
    fn simple_type_eq_symmetric(a in arb_simple_type(), b in arb_simple_type()) {
        prop_assert_eq!(a == b, b == a);
    }

    #[test]
    fn simple_type_eq_transitive(
        a in arb_simple_type(),
        b in arb_simple_type(),
        c in arb_simple_type(),
    ) {
        if a == b && b == c {
            prop_assert_eq!(a, c);
        }
    }
}

// ---------------------------------------------------------------------------
// 5.15 left_adj panics on i8::MIN; right_adj panics on i8::MAX
// ---------------------------------------------------------------------------

#[test]
#[should_panic(expected = "adjoint underflow")]
fn left_adj_panics_on_min() {
    let t = SimpleType {
        base: TypeId::Dir,
        adjoint: i8::MIN,
    };
    let _ = t.left_adj();
}

#[test]
#[should_panic(expected = "adjoint overflow")]
fn right_adj_panics_on_max() {
    let t = SimpleType {
        base: TypeId::Dir,
        adjoint: i8::MAX,
    };
    let _ = t.right_adj();
}

// ---------------------------------------------------------------------------
// 5.16 can_contract never panics (exhaustive for extreme values)
// ---------------------------------------------------------------------------

#[test]
fn can_contract_never_panics() {
    let bases = [TypeId::Dir, TypeId::Ag];
    let extremes = [i8::MIN, i8::MIN + 1, -1, 0, 1, i8::MAX - 1, i8::MAX];
    for &b1 in &bases {
        for &b2 in &bases {
            for &a1 in &extremes {
                for &a2 in &extremes {
                    // Must not panic — result doesn't matter.
                    let _ = can_contract(
                        SimpleType {
                            base: b1,
                            adjoint: a1,
                        },
                        SimpleType {
                            base: b2,
                            adjoint: a2,
                        },
                    );
                }
            }
        }
    }
}
