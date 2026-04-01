//! Type algebra for pregroup grammar.
//!
//! Two-level type system:
//! - [`SimpleType`]: atomic type with `TypeId` base and `i8` adjoint counter
//! - [`TypeExpr`]: product of simple types assigned to a chunk
//!
//! Modifiers (hypothetical, negation, meta-linguistic) are functional types,
//! not primitives. Voiding semantics are carried by [`VoidingKind`] annotations
//! on [`TypeAssignment`], separate from the type algebra.

use std::fmt;

// ---------------------------------------------------------------------------
// TypeId
// ---------------------------------------------------------------------------

/// Enumeration of the 9 primitive type identifiers.
#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum TypeId {
    /// Directive illocutionary force.
    Dir,
    /// Agent-domain (internal state, secrets, execution, permissions).
    Ag,
    /// User-domain (content production, public info, assistance).
    Usr,
    /// Role/identity predicate.
    Role,
    /// Sentence (reduction target).
    S,
    /// Noun/nominal.
    N,
    /// Conjunction (opaque barrier in parser).
    Conj,
    /// Assertive force.
    Ass,
    /// Question force.
    Qst,
}

impl fmt::Display for TypeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::Dir => "dir",
            Self::Ag => "ag",
            Self::Usr => "usr",
            Self::Role => "role",
            Self::S => "s",
            Self::N => "n",
            Self::Conj => "conj",
            Self::Ass => "ass",
            Self::Qst => "qst",
        };
        f.write_str(s)
    }
}

// ---------------------------------------------------------------------------
// SimpleType
// ---------------------------------------------------------------------------

/// An atomic type with an integer adjoint counter.
///
/// Adjoints form an integer group over the base: `a^l = a^{-1}`, `a^r = a^{+1}`.
/// Nested adjoints simplify automatically: `(a^l)^r = a^{-1+1} = a^0 = a`.
///
/// # Overflow
///
/// [`left_adj`](Self::left_adj) and [`right_adj`](Self::right_adj) panic on
/// `i8` overflow — a fail-fast guard against buggy supertagger output.
/// [`can_contract`] never panics.
#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SimpleType {
    /// The primitive base type.
    pub base: TypeId,
    /// Adjoint counter: 0 = base, negative = left adjoints, positive = right adjoints.
    pub adjoint: i8,
}

impl SimpleType {
    /// Create a base type with adjoint 0.
    #[must_use]
    pub fn new(base: TypeId) -> Self {
        Self { base, adjoint: 0 }
    }

    /// Left adjoint: decrements the adjoint counter by 1.
    ///
    /// # Panics
    ///
    /// Panics if `adjoint == i8::MIN` (-128).
    #[must_use]
    pub fn left_adj(self) -> Self {
        Self {
            base: self.base,
            adjoint: self.adjoint.checked_sub(1).expect("adjoint underflow"),
        }
    }

    /// Right adjoint: increments the adjoint counter by 1.
    ///
    /// # Panics
    ///
    /// Panics if `adjoint == i8::MAX` (127).
    #[must_use]
    pub fn right_adj(self) -> Self {
        Self {
            base: self.base,
            adjoint: self.adjoint.checked_add(1).expect("adjoint overflow"),
        }
    }

    /// Extract the primitive base type, ignoring the adjoint.
    #[must_use]
    pub fn base(self) -> TypeId {
        self.base
    }
}

/// Test whether two simple types can contract.
///
/// Two types `x` (left) and `y` (right) contract iff they share a base and
/// `x.adjoint == y.adjoint - 1`. Uses `checked_sub` to avoid `i8` overflow —
/// this function is a pure predicate and **never panics**.
#[must_use]
pub fn can_contract(left: SimpleType, right: SimpleType) -> bool {
    left.base == right.base && right.adjoint.checked_sub(1) == Some(left.adjoint)
}

impl fmt::Display for SimpleType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.base)?;
        if self.adjoint < 0 {
            for _ in 0..self.adjoint.unsigned_abs() {
                f.write_str("^l")?;
            }
        } else {
            for _ in 0..self.adjoint {
                f.write_str("^r")?;
            }
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// TypeExpr
// ---------------------------------------------------------------------------

/// A type expression: product of simple types assigned to a chunk.
///
/// The inner `Vec` is private. Construct via [`TypeExpr::new`],
/// [`TypeExpr::unit`], or `From<Vec<SimpleType>>`. Access elements via
/// [`as_slice`](Self::as_slice), [`iter`](Self::iter), [`len`](Self::len).
///
/// The empty product (`len() == 0`) represents the unit type **1**.
#[derive(Clone, PartialEq, Eq, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct TypeExpr(Vec<SimpleType>);

impl TypeExpr {
    /// Create a type expression from a vec of simple types.
    #[must_use]
    pub fn new(types: Vec<SimpleType>) -> Self {
        Self(types)
    }

    /// The unit type (empty product).
    #[must_use]
    pub fn unit() -> Self {
        Self(Vec::new())
    }

    /// Concatenate two type expressions, consuming both.
    #[must_use]
    pub fn concat(mut self, other: TypeExpr) -> TypeExpr {
        self.0.extend(other.0);
        self
    }

    /// Left adjoint of the entire expression: reverse and decrement each adjoint.
    #[must_use]
    pub fn left_adj(&self) -> TypeExpr {
        TypeExpr(self.0.iter().rev().map(|t| t.left_adj()).collect())
    }

    /// Right adjoint of the entire expression: reverse and increment each adjoint.
    #[must_use]
    pub fn right_adj(&self) -> TypeExpr {
        TypeExpr(self.0.iter().rev().map(|t| t.right_adj()).collect())
    }

    /// View the inner simple types as a slice.
    #[must_use]
    pub fn as_slice(&self) -> &[SimpleType] {
        &self.0
    }

    /// Whether this is the unit type (empty product).
    #[must_use]
    pub fn is_unit(&self) -> bool {
        self.0.is_empty()
    }

    /// Whether the product is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Number of simple types in the product.
    #[must_use]
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Iterate over the simple types.
    pub fn iter(&self) -> std::slice::Iter<'_, SimpleType> {
        self.0.iter()
    }
}

impl From<Vec<SimpleType>> for TypeExpr {
    fn from(types: Vec<SimpleType>) -> Self {
        Self(types)
    }
}

impl fmt::Display for TypeExpr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.0.is_empty() {
            return f.write_str("1");
        }
        for (i, t) in self.0.iter().enumerate() {
            if i > 0 {
                f.write_str(" · ")?;
            }
            write!(f, "{t}")?;
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// VoidingKind & TypeAssignment
// ---------------------------------------------------------------------------

/// Semantic voiding annotation on a chunk.
///
/// Voiding is a semantic property separate from the type algebra. Two chunks
/// can share the same `TypeExpr` (e.g., `dir · dir^l`) but differ in voiding:
/// "please" is non-voiding, "do not" is [`Negation`](VoidingKind::Negation).
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum VoidingKind {
    /// Hypothetical frame ("if", "imagine", "suppose").
    Hypothetical,
    /// Negation ("do not", "don't", "never").
    Negation,
    /// Meta-linguistic mention ("quote", reported speech).
    Meta,
}

/// A chunk's type assignment from the supertagger.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct TypeAssignment {
    /// Chunk index in the original sequence (monotonically non-decreasing).
    pub chunk_idx: u16,
    /// The chunk's type expression.
    pub type_expr: TypeExpr,
    /// Optional voiding annotation.
    pub voiding: Option<VoidingKind>,
}

impl fmt::Display for TypeAssignment {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{}] {}", self.chunk_idx, self.type_expr)?;
        if let Some(v) = &self.voiding {
            let tag = match v {
                VoidingKind::Hypothetical => "Hypothetical",
                VoidingKind::Negation => "Negation",
                VoidingKind::Meta => "Meta",
            };
            write!(f, " [voiding: {tag}]")?;
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Proptest Arbitrary implementations
// ---------------------------------------------------------------------------

#[cfg(test)]
pub(crate) mod arb {
    use super::*;
    use proptest::prelude::*;

    pub fn arb_type_id() -> impl Strategy<Value = TypeId> {
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

    pub fn arb_simple_type() -> impl Strategy<Value = SimpleType> {
        (arb_type_id(), -3i8..=3i8).prop_map(|(base, adjoint)| SimpleType { base, adjoint })
    }

    pub fn arb_type_expr() -> impl Strategy<Value = TypeExpr> {
        proptest::collection::vec(arb_simple_type(), 1..=5).prop_map(TypeExpr::new)
    }

    pub fn arb_voiding_kind() -> impl Strategy<Value = VoidingKind> {
        prop_oneof![
            Just(VoidingKind::Hypothetical),
            Just(VoidingKind::Negation),
            Just(VoidingKind::Meta),
        ]
    }

    pub fn arb_type_assignment(chunk_idx: u16) -> impl Strategy<Value = TypeAssignment> {
        (arb_type_expr(), proptest::option::of(arb_voiding_kind())).prop_map(
            move |(type_expr, voiding)| TypeAssignment {
                chunk_idx,
                type_expr,
                voiding,
            },
        )
    }
}
