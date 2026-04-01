//! Integration tests: full parse → scope check pipeline.

mod test_fixtures;

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

fn assert_verdict(assignments: Vec<TypeAssignment>, expect_injection: bool) {
    let input = ParseInput(assignments.clone());
    let graph = parse(&input, None);
    let verdict = check_scope(&graph, &assignments);
    if expect_injection {
        assert!(
            matches!(verdict, Verdict::Injection { .. }),
            "Expected Injection, got Clean.\nGraph:\n{graph}"
        );
    } else {
        assert_eq!(
            verdict,
            Verdict::Clean,
            "Expected Clean, got Injection.\nGraph:\n{graph}"
        );
    }
}

// ---------------------------------------------------------------------------
// 11.3-11.11: Individual test cases
// ---------------------------------------------------------------------------

#[test]
fn inj_001_direct_override() {
    assert_verdict(
        vec![ta(
            0,
            vec![st(TypeId::Dir, 0), st(TypeId::Ag, -1), st(TypeId::Ag, 0)],
            None,
        )],
        true,
    );
}

#[test]
fn inj_002_stop_ignore() {
    assert_verdict(
        vec![
            ta(0, vec![st(TypeId::Dir, 0)], None),
            ta(
                1,
                vec![st(TypeId::Dir, 0), st(TypeId::Ag, -1), st(TypeId::Ag, 0)],
                None,
            ),
            ta(
                2,
                vec![st(TypeId::Dir, 0), st(TypeId::Usr, -1), st(TypeId::Usr, 0)],
                None,
            ),
        ],
        true,
    );
}

#[test]
fn inj_004_role_hijack() {
    assert_verdict(
        vec![
            ta(
                0,
                vec![st(TypeId::Role, 0), st(TypeId::Ag, -1), st(TypeId::Ag, 0)],
                None,
            ),
            ta(1, vec![st(TypeId::Qst, 0), st(TypeId::Usr, -1)], None),
        ],
        true,
    );
}

#[test]
fn inj_013_threat() {
    assert_verdict(
        vec![
            ta(
                0,
                vec![st(TypeId::Dir, 0), st(TypeId::Ag, -1), st(TypeId::Ag, 0)],
                None,
            ),
            ta(1, vec![st(TypeId::Conj, 0)], None),
            ta(2, vec![st(TypeId::Ass, 0)], None),
        ],
        true,
    );
}

#[test]
fn ben_007_meta_discussion() {
    assert_verdict(
        vec![
            ta(
                0,
                vec![st(TypeId::N, 0), st(TypeId::N, -1)],
                Some(VoidingKind::Meta),
            ),
            ta(
                1,
                vec![st(TypeId::N, 0), st(TypeId::Qst, 0), st(TypeId::Usr, -1)],
                None,
            ),
        ],
        false,
    );
}

#[test]
fn ben_009_hypothetical() {
    assert_verdict(
        vec![
            ta(
                0,
                vec![st(TypeId::S, 0), st(TypeId::S, -1)],
                Some(VoidingKind::Hypothetical),
            ),
            ta(
                1,
                vec![
                    st(TypeId::S, 0),
                    st(TypeId::Dir, 0),
                    st(TypeId::Ag, -1),
                    st(TypeId::Ag, 0),
                ],
                None,
            ),
            ta(2, vec![st(TypeId::Qst, 0), st(TypeId::Usr, -1)], None),
        ],
        false,
    );
}

#[test]
fn ben_011_quoted_content() {
    assert_verdict(
        vec![
            ta(0, vec![st(TypeId::Ass, 0)], None),
            ta(1, vec![st(TypeId::N, 1), st(TypeId::N, 0)], None),
            ta(
                2,
                vec![
                    st(TypeId::N, 0),
                    st(TypeId::Dir, 0),
                    st(TypeId::Ag, -1),
                    st(TypeId::Ag, 0),
                ],
                Some(VoidingKind::Meta),
            ),
            ta(3, vec![st(TypeId::N, 1), st(TypeId::N, 0)], None),
        ],
        false,
    );
}

#[test]
fn ben_015_user_domain_directive() {
    assert_verdict(
        vec![
            ta(0, vec![st(TypeId::Dir, 0), st(TypeId::Usr, -1)], None),
            ta(1, vec![st(TypeId::Usr, 0)], None),
        ],
        false,
    );
}

#[test]
fn ben_018_negated_directive() {
    assert_verdict(
        vec![
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
            ta(2, vec![st(TypeId::Dir, 0), st(TypeId::Usr, -1)], None),
            ta(3, vec![st(TypeId::Usr, 0)], None),
        ],
        false,
    );
}

// ---------------------------------------------------------------------------
// 11.12: Security — Pass 2 surfaces injection
// ---------------------------------------------------------------------------

#[test]
fn security_pass2_surfaces_injection() {
    // [n^l, dir · ag^l, n, ag] — crossing forces exclusive choice.
    // Pass 1 picks n^l↔n (no injection). Pass 2 surfaces ag^l↔ag.
    assert_verdict(
        vec![
            ta(0, vec![st(TypeId::N, -1)], None),
            ta(1, vec![st(TypeId::Dir, 0), st(TypeId::Ag, -1)], None),
            ta(2, vec![st(TypeId::N, 0)], None),
            ta(3, vec![st(TypeId::Ag, 0)], None),
        ],
        true,
    );
}

// ---------------------------------------------------------------------------
// 11.13: Run all fixtures
// ---------------------------------------------------------------------------

#[test]
fn all_fixtures_pass() {
    for fixture in test_fixtures::all_fixtures() {
        let input = ParseInput(fixture.assignments.clone());
        let graph = parse(&input, None);
        let verdict = check_scope(&graph, &fixture.assignments);
        if fixture.expected_injection {
            assert!(
                matches!(verdict, Verdict::Injection { .. }),
                "{}: expected Injection, got Clean.\nGraph:\n{graph}",
                fixture.name,
            );
        } else {
            assert_eq!(
                verdict,
                Verdict::Clean,
                "{}: expected Clean, got Injection.\nGraph:\n{graph}",
                fixture.name,
            );
        }
    }
}
