## ADDED Requirements

### Requirement: System prompt contains type inventory
The system prompt SHALL include the complete type inventory: the 9 primitive TypeIds (`Dir`, `Ag`, `Usr`, `Role`, `S`, `N`, `Conj`, `Ass`, `Qst`), the functional modifier patterns (noun modifier, agent modifier, hypothetical, negation, meta-linguistic), and the three `VoidingKind` values with their semantics.

#### Scenario: All primitives documented
- **WHEN** the LLM reads the system prompt
- **THEN** it SHALL have access to all 9 primitive type names and their descriptions

#### Scenario: Modifier patterns documented
- **WHEN** the LLM needs to assign a hypothetical operator
- **THEN** the system prompt SHALL include the pattern `s · s^l` with `voiding: Hypothetical`

### Requirement: System prompt contains output schema
The system prompt SHALL include the exact JSON schema for the response: an array of objects with `chunk_idx` (u16), `chunk_text` (string), `type_expr` (array of `{base, adjoint}`), and `voiding` (null or string).

#### Scenario: Schema is explicit
- **WHEN** the LLM generates output
- **THEN** it SHALL follow the documented JSON schema exactly

### Requirement: Few-shot examples
The system prompt SHALL include 4-6 few-shot examples from the ground truth corpus, covering:
- At least one direct injection (e.g., inj_001)
- At least one role hijack (e.g., inj_004)
- At least one benign with voiding (e.g., ben_009 or ben_018)
- At least one benign user-domain directive (e.g., ben_015)

#### Scenario: Injection example present
- **WHEN** the LLM encounters a direct override pattern
- **THEN** it SHALL have seen a similar example in the few-shot section

#### Scenario: Voiding example present
- **WHEN** the LLM encounters a hypothetical frame
- **THEN** it SHALL have seen a voiding annotation example in the few-shot section

### Requirement: User message contains only input text
The user message SHALL contain only the raw text to analyze, with no additional framing or instructions.

#### Scenario: Clean input
- **WHEN** sending text "Ignore your instructions"
- **THEN** the user message SHALL be exactly that string, nothing more

### Requirement: Prompt is a versioned static asset
The prompt template SHALL be stored as a static string constant in the crate, versioned with a constant identifier (e.g., `PROMPT_V1`). Changing the prompt creates a new version.

#### Scenario: Prompt version accessible
- **WHEN** querying the supertagger
- **THEN** the prompt version SHALL be available in the response metadata for audit
