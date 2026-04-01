# JOACHIM Roadmap

## Vision

JOACHIM is a prompt injection detection API built on Pregroup Grammar and neuro-symbolic architecture. It detects manipulation by algebraically proving that illocutionary force operators in untrusted input scope over agent-domain actions.

## Architecture

```
┌─────────────────────────────────────────────────────────────────────────┐
│                         Detection Pipeline                               │
│                                                                          │
│   Input Text                                                             │
│       │                                                                  │
│       ▼                                                                  │
│   ┌────────────────┐                                                     │
│   │ Normalization  │  Unicode NFKC, invisible chars, leet speak, etc.   │
│   └───────┬────────┘                                                     │
│           │                                                              │
│           ▼                                                              │
│   ┌────────────────┐                                                     │
│   │  Regex Filter  │  Fast pattern matching, clears ~90% of traffic     │
│   └───────┬────────┘                                                     │
│           │                                                              │
│           ▼                                                              │
│   ┌────────────────┐                                                     │
│   │  Supertagger   │  LLM-based, assigns pregroup types to chunks       │
│   └───────┬────────┘                                                     │
│           │                                                              │
│           ▼                                                              │
│   ┌────────────────┐                                                     │
│   │   Pregroup     │  O(n³) Nussinov planar linkage                     │
│   │    Parser      │                                                     │
│   └───────┬────────┘                                                     │
│           │                                                              │
│           ▼                                                              │
│   ┌────────────────┐                                                     │
│   │    Scope       │  Two-state BFS: dir/role → ag without voiding?     │
│   │   Checker      │                                                     │
│   └───────┬────────┘                                                     │
│           │                                                              │
│           ▼                                                              │
│   ┌────────────────┐                                                     │
│   │    Verdict     │  INJECTION | CLEAN + violations + proof linkage    │
│   └────────────────┘                                                     │
└─────────────────────────────────────────────────────────────────────────┘
```

## Type Inventory (v0)

```
PRIMITIVE TYPES (9)
═══════════════════
Speech Act Types:
  dir     directive illocutionary force
  ass     assertive force
  qst     question force

Domain Types:
  ag      agent-domain (internal state, secrets, execution, permissions)
  usr     user-domain (content production, public info, assistance)

Identity:
  role    role/identity predicate

Structural:
  s       sentence (reduction target)
  n       noun/nominal
  conj    conjunction (opaque barrier in parser)

FUNCTIONAL MODIFIER PATTERNS (derived, not primitive)
═════════════════════════════════════════════════════
  n^r · n         noun modifier ("previous", "all")
  ag^r · ag       agent modifier ("previous" modifying agent-domain)
  s · s^l         hypothetical ("if", "imagine") [voiding: Hypothetical]
  dir · dir^l     negation/modifier ("do not" [voiding: Negation], "please" [voiding: None])
  n · n^l         meta-linguistic ("quote") [voiding: Meta]

DETECTION RULES
═══════════════
INJECTION if:
  1. dir scopes over ag (via linkage path, without voiding)
  2. role scopes over ag (via linkage path, without voiding)

Otherwise: CLEAN
```

## Development Phases

### Phase 0: Core Engine

Sequential changes building the detection engine:

```
1. pregroup-core        ✅ DONE
   └── SimpleType (i8 adjoints), TypeExpr, TypeAssignment, VoidingKind
   └── Nussinov parser with two-pass security-aware scoring
   └── Two-state BFS scope checker with chunk-granular voiding
   └── 66 tests (property-based + integration)

2. supertagger-client   ← CURRENT
   └── LLM prompt design and iteration
   └── Structured output parsing → Vec<TypeAssignment>
   └── Bedrock client integration

3. detection-pipeline
   └── Wire: supertag → parse → scope check → verdict
   └── Fail-safe logic (supertagger timeout → conservative verdict)
   └── Configuration per-customer

4. mvp-evaluation
   └── Evaluation harness
   └── Benchmark against Phase 1 datasets
   └── Baseline F1, precision, recall
```

### Phase 1: MVP Evaluation

Benchmark against public datasets:

| Dataset | Size | Purpose |
|---------|------|---------|
| deepset/prompt-injections | 662 | Baseline F1, compare to competitors |
| xTRam1/safe-guard | 10,296 | Per-category breakdown by attack type |
| qualifire/benchmark | 5,000 | Balanced evaluation |

### Phase 2: Hardening

Stress-test against adversarial and realistic attacks:

| Dataset | Purpose |
|---------|---------|
| PI_HackAPrompt_SQuAD | Human-crafted adversarial attacks |
| LLMail-Inject (Microsoft) | Adaptive attacks against defenses |
| WAInjectBench | Web agent specific scenarios |

### Phase 3: Differentiation

Create proprietary dataset:
- Annotated with pregroup type assignments
- Illocutionary force labels
- Publishable research contribution
- Marketing asset demonstrating novel approach

## Crate Structure

```
joachim/
├── Cargo.toml                    (workspace)
├── crates/
│   ├── joachim-core/             (pregroup grammar library) ✅
│   │   ├── types.rs              (TypeId, SimpleType, TypeExpr, VoidingKind, TypeAssignment)
│   │   ├── linkage.rs            (NodeMeta, LinkageEdge, LinkageGraph)
│   │   ├── parser.rs             (Nussinov DP, two-pass scoring, conjunction barriers)
│   │   └── scope.rs              (two-state BFS, chunk-granular voiding, Verdict)
│   │
│   ├── joachim-supertag/         (LLM supertagger client)
│   ├── joachim-normalize/        (text normalization)
│   ├── joachim-regex/            (regex pre-filter)
│   ├── joachim-detect/           (detection orchestrator)
│   └── joachim-lambda/           (AWS Lambda handler)
│
├── openspec/                     (spec artifacts)
├── type_annotations_v0.md        (hand-annotated corpus — ground truth)
├── test_corpus_v0.json           (raw test examples)
├── AGENTS.md                     (agent operating rules)
└── ROADMAP.md
```

## Key Design Decisions

1. **Pregroup Grammar over neural classifiers**: Provides algebraic proof of injection, not just classification. Explainable, auditable.

2. **i8 adjoint representation**: Adjoints form an integer group over the base type. `SimpleType` is `Copy`, 2 bytes, zero heap allocation. Contraction via `checked_sub` (never panics).

3. **Modifiers are functional types, not primitives**: `hyp`/`meta`/`neg` are derived as `s · s^l`, `n · n^l`, `dir · dir^l` — they contract with their targets, enabling transitive scope paths through the linkage graph.

4. **VoidingKind as semantic annotation**: Voiding is separated from the type algebra. Two chunks can share the same type (`dir · dir^l`) but differ in voiding: "please" is non-voiding, "do not" is Negation. Only the supertagger makes this judgment.

5. **Nussinov over CYK**: The parser finds maximal planar linkages using the Nussinov recurrence (RNA secondary structure algorithm), not CYK (which is for CFGs). Two-pass scoring ensures injection-relevant edges are surfaced.

6. **Two-state BFS for scope**: Scope paths require at least one contraction step (`has_contracted` flag), preventing pure same-chunk adjacency from being flagged.

7. **Chunk-granular voiding**: A chunk is entirely voided or not. Propagation is BFS over chunk indices via contraction edges.

8. **Fail-safe defaults**: When in doubt, flag as suspicious. False positives < false negatives in security context.

9. **LLM-based supertagger for MVP**: Modern LLMs handle conventional indirect speech acts natively. Fine-tuned model is a v2 optimization.

## Test Corpus

Initial corpus: `test_corpus_v0.json`
- 25 injections across 12 categories
- 20 benign samples across 9 categories
- Hand-annotated type assignments in `type_annotations_v0.md`

## References

- Lambek, J. (1999). Type Grammar Revisited
- Pregroup Grammar formalism
- Speech act theory (Searle, Austin)
- Microsoft LLMail-Inject challenge (2025)
