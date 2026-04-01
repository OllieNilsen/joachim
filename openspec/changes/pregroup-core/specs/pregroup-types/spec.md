## ADDED Requirements

### Requirement: Primitive type representation
The system SHALL represent primitive types as an enumeration of 9 values: `dir` (directive), `ag` (agent-domain), `usr` (user-domain), `role` (identity), `s` (sentence), `n` (noun), `conj` (conjunction), `ass` (assertive), `qst` (question).

Modifiers (including hypothetical, meta-linguistic, and negation operators) are NOT primitive types. Following categorial grammar convention, modifiers are represented as functional types over their target (e.g., `n^r · n` for a noun modifier, `dir · dir^l` for a negation operator).

#### Scenario: All primitive types are representable
- **WHEN** a type assignment uses any primitive type from the inventory
- **THEN** the system SHALL be able to construct a valid `SimpleType` value with `adjoint = 0`

#### Scenario: Modifier as functional type
- **WHEN** the supertagger assigns modifier semantics (e.g., for "previous" modifying "instructions")
- **THEN** this SHALL be represented as a `TypeExpr` containing `ag^r · ag`, not a primitive `mod` type

#### Scenario: Voiding operator as functional type
- **WHEN** representing a hypothetical operator ("if") voiding a sentence
- **THEN** this SHALL be represented as a `TypeExpr` containing `s · s^l`, not a primitive `hyp` type

### Requirement: Two-level type representation
The system SHALL represent types at two levels:
- `SimpleType`: An atomic type defined by a `TypeId` base and an `i8` adjoint count (e.g., `dir` is `adjoint=0`, `ag^l` is `adjoint=-1`, `(n^r)^l` is `adjoint=0`).
- `TypeExpr`: A product of simple types assigned to a chunk. The inner `Vec<SimpleType>` is private; elements are accessed via `as_slice()`, `iter()`, `len()`. Construct via `TypeExpr::new(vec)`, `TypeExpr::unit()`, or `From<Vec<SimpleType>>`.

The empty `TypeExpr` (no simple types) represents the unit type `1`.

#### Scenario: Construct simple type
- **WHEN** representing the type of a directive
- **THEN** the system SHALL construct `SimpleType { base: Dir, adjoint: 0 }`

#### Scenario: Construct type expression
- **WHEN** representing a directive seeking an agent-domain complement
- **THEN** the system SHALL construct `TypeExpr([SimpleType { base: Dir, adjoint: 0 }, SimpleType { base: Ag, adjoint: -1 }])`

#### Scenario: Unit type as empty product
- **WHEN** a contraction produces the unit type
- **THEN** the result SHALL be `TypeExpr(vec![])` (empty product)

### Requirement: Left adjoint operation
The system SHALL support left adjoint operations written as `a^l` for any `SimpleType` `a`. Left adjoints decrement the integer adjoint counter by 1. The operation SHALL panic on i8 underflow (adjoint == -128).

#### Scenario: Construct left adjoint
- **WHEN** given a SimpleType `dir` (`adjoint=0`)
- **THEN** the system SHALL produce `dir^l` (`adjoint=-1`)

#### Scenario: Nested adjoints simplify
- **WHEN** given a left adjoint type `dir^l` (`adjoint=-1`)
- **THEN** the system SHALL produce `(dir^l)^r` as `dir` (`adjoint=-1+1=0`)

### Requirement: Right adjoint operation
The system SHALL support right adjoint operations written as `a^r` for any `SimpleType` `a`. Right adjoints increment the integer adjoint counter by 1. The operation SHALL panic on i8 overflow (adjoint == 127).

#### Scenario: Construct right adjoint
- **WHEN** given a SimpleType `ag` (`adjoint=0`)
- **THEN** the system SHALL produce `ag^r` (`adjoint=1`)

### Requirement: Contraction rules
The system SHALL implement contraction via a single formula. Two SimpleTypes `x` (left position) and `y` (right position) can contract if and only if:

```rust
fn can_contract(x: SimpleType, y: SimpleType) -> bool {
    x.base == y.base && y.adjoint.checked_sub(1).map_or(false, |r| x.adjoint == r)
}
```

The `checked_sub` prevents i8 underflow. If `y.adjoint == i8::MIN`, `can_contract` returns `false`. This function is a pure predicate and SHALL never panic.

This formula unifies both contraction rules:
- Left contraction `a^l · a → 1`: `(base=A, adj=-1)` beside `(base=A, adj=0)` → `-1 == 0 - 1` ✓
- Right contraction `a · a^r → 1`: `(base=A, adj=0)` beside `(base=A, adj=1)` → `0 == 1 - 1` ✓
- Higher adjoints `a^{ll} · a^l → 1`: `(base=A, adj=-2)` beside `(base=A, adj=-1)` → `-2 == -1 - 1` ✓

#### Scenario: Left contraction succeeds
- **WHEN** the parser encounters adjacent simple types `ag^l` (`adjoint=-1`) followed by `ag` (`adjoint=0`)
- **THEN** the system SHALL recognize they can contract (`-1 == 0 - 1`)

#### Scenario: Right contraction succeeds
- **WHEN** the parser encounters adjacent simple types `dir` (`adjoint=0`) followed by `dir^r` (`adjoint=1`)
- **THEN** the system SHALL recognize they can contract (`0 == 1 - 1`)

#### Scenario: Higher adjoint contraction succeeds
- **WHEN** the parser encounters `n^{ll}` (`adjoint=-2`) followed by `n^l` (`adjoint=-1`)
- **THEN** the system SHALL recognize they can contract (`-2 == -1 - 1`)

#### Scenario: Contraction type mismatch
- **WHEN** the parser encounters adjacent simple types `ag^l` followed by `usr`
- **THEN** the system SHALL NOT contract them (base types differ)

#### Scenario: Contraction adjoint mismatch
- **WHEN** the parser encounters `ag` (`adjoint=0`) followed by `ag` (`adjoint=0`)
- **THEN** the system SHALL NOT contract them (`0 != 0 - 1`)

### Requirement: SimpleType equality
The system SHALL implement structural equality for `SimpleType` by comparing the `base` and `adjoint` fields directly. Because nested adjoints are automatically simplified by the i8 representation, equality is trivial field comparison.

#### Scenario: Equal primitives
- **WHEN** comparing `dir` to `dir`
- **THEN** they SHALL be equal

#### Scenario: Unequal adjoints
- **WHEN** comparing `dir^l` to `dir^r`
- **THEN** they SHALL NOT be equal (`adjoint` differs: -1 vs 1)

#### Scenario: Nested adjoint simplification
- **WHEN** comparing `(ag^l)^r` to `ag`
- **THEN** they SHALL be equal (`adjoint=-1+1=0`)

#### Scenario: Double adjoint is distinct
- **WHEN** comparing `(a^l)^l` (`adjoint=-2`) to `a` (`adjoint=0`)
- **THEN** they SHALL NOT be equal

### Requirement: SimpleType display
The system SHALL provide human-readable string representation of simple types based on the integer adjoint count. If `adjoint == 0`, display just the primitive name. If `adjoint < 0`, append `^l` repeated `abs(adjoint)` times. If `adjoint > 0`, append `^r` repeated `adjoint` times.

#### Scenario: Display simple adjoint
- **WHEN** displaying `SimpleType { base: Ag, adjoint: -1 }`
- **THEN** the output SHALL be `ag^l`

#### Scenario: Display double adjoint
- **WHEN** displaying `SimpleType { base: N, adjoint: 2 }`
- **THEN** the output SHALL be `n^r^r`

### Requirement: TypeExpr display
The system SHALL display type expressions as `·`-separated simple types.

#### Scenario: Display product type
- **WHEN** displaying `TypeExpr([dir, ag^l])`
- **THEN** the output SHALL be `dir · ag^l`

#### Scenario: Display unit
- **WHEN** displaying `TypeExpr([])`
- **THEN** the output SHALL be `1`

### Requirement: Extracting primitive base
The system SHALL provide a method to extract the primitive `TypeId` base from any `SimpleType`, effectively ignoring the `adjoint` count.

#### Scenario: Base of adjoint
- **WHEN** extracting the base of `ag^l`
- **THEN** the result SHALL be `TypeId::Ag`
