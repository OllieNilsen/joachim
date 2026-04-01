//! Ground truth test fixtures from `type_annotations_v0.md`.
//!
//! Single source of truth for integration tests. Each fixture encodes a
//! `Vec<TypeAssignment>` and the expected `Verdict`.

use joachim_core::scope::Verdict;
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

pub struct Fixture {
    pub name: &'static str,
    pub assignments: Vec<TypeAssignment>,
    pub expected_injection: bool,
}

pub fn all_fixtures() -> Vec<Fixture> {
    vec![
        // --- INJECTION ---
        Fixture {
            name: "inj_001: direct override",
            assignments: vec![ta(
                0,
                vec![st(TypeId::Dir, 0), st(TypeId::Ag, -1), st(TypeId::Ag, 0)],
                None,
            )],
            expected_injection: true,
        },
        Fixture {
            name: "inj_002: stop + ignore",
            assignments: vec![
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
            expected_injection: true,
        },
        Fixture {
            name: "inj_004: role hijack",
            assignments: vec![
                ta(
                    0,
                    vec![st(TypeId::Role, 0), st(TypeId::Ag, -1), st(TypeId::Ag, 0)],
                    None,
                ),
                ta(1, vec![st(TypeId::Qst, 0), st(TypeId::Usr, -1)], None),
            ],
            expected_injection: true,
        },
        Fixture {
            name: "inj_013: threat/coercion",
            assignments: vec![
                ta(
                    0,
                    vec![st(TypeId::Dir, 0), st(TypeId::Ag, -1), st(TypeId::Ag, 0)],
                    None,
                ),
                ta(1, vec![st(TypeId::Conj, 0)], None),
                ta(2, vec![st(TypeId::Ass, 0)], None),
            ],
            expected_injection: true,
        },
        // --- BENIGN ---
        Fixture {
            name: "ben_007: meta discussion",
            assignments: vec![
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
            expected_injection: false,
        },
        Fixture {
            name: "ben_009: hypothetical",
            assignments: vec![
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
            expected_injection: false,
        },
        Fixture {
            name: "ben_011: quoted content",
            assignments: vec![
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
            expected_injection: false,
        },
        Fixture {
            name: "ben_015: user-domain directive",
            assignments: vec![
                ta(0, vec![st(TypeId::Dir, 0), st(TypeId::Usr, -1)], None),
                ta(1, vec![st(TypeId::Usr, 0)], None),
            ],
            expected_injection: false,
        },
        Fixture {
            name: "ben_018: negated directive",
            assignments: vec![
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
            expected_injection: false,
        },
    ]
}
