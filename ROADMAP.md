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
│   │   Pregroup     │  O(n³) planar type reduction                       │
│   │    Parser      │                                                     │
│   └───────┬────────┘                                                     │
│           │                                                              │
│           ▼                                                              │
│   ┌────────────────┐                                                     │
│   │    Scope       │  Does dir/role scope over ag without voiding?      │
│   │   Checker      │                                                     │
│   └───────┬────────┘                                                     │
│           │                                                              │
│           ▼                                                              │
│   ┌────────────────┐                                                     │
│   │    Verdict     │  INJECTION | CLEAN + confidence + proof            │
│   └────────────────┘                                                     │
└─────────────────────────────────────────────────────────────────────────┘
```

## Type Inventory (v0)

```
BASIC TYPES
═══════════
Speech Act Types:
  dir     directive illocutionary force
  ass     assertive force
  qst     question force

Domain Types:
  ag      agent-domain (internal state, secrets, execution, permissions)
  usr     user-domain (content production, public info, assistance)

Scope Modifiers:
  hyp     hypothetical operator
  meta    meta-linguistic operator (quotation, mention)
  neg     negation operator

Identity:
  role    role/identity predicate

Structural:
  s       sentence (reduction target)
  n       noun/nominal
  mod     modifier
  conj    conjunction

DETECTION RULES
═══════════════
INJECTION if:
  1. dir scopes over ag (without hyp/meta/neg voiding)
  2. role assigns to ag (without hyp/meta/neg voiding)

Otherwise: CLEAN
```

## Development Phases

### Phase 0: Core Engine (Current)

Sequential changes building the detection engine:

```
1. pregroup-core        ← START HERE
   └── PregroupType, Parser, ScopeChecker in Rust
   └── Pure, zero external dependencies
   └── Independently testable and fuzzable
   
2. supertagger-client
   └── LLM prompt design and iteration
   └── Structured output parsing
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

## Crate Structure (Target)

```
joachim/
├── Cargo.toml                    (workspace)
├── crates/
│   ├── joachim-core/             (pregroup grammar library)
│   │   ├── types.rs              (PregroupType, TypeAssignment)
│   │   ├── parser.rs             (CYK-style planar reduction)
│   │   ├── scope.rs              (ScopeChecker)
│   │   └── proof.rs              (proof representation)
│   │
│   ├── joachim-normalize/        (text normalization)
│   ├── joachim-regex/            (regex pre-filter)
│   ├── joachim-supertag/         (LLM supertagger client)
│   ├── joachim-detect/           (detection orchestrator)
│   └── joachim-lambda/           (AWS Lambda handler)
│
├── iac/                          (Infrastructure as Code)
└── tests/
    ├── fixtures/                 (test corpus)
    └── integration/
```

## Key Design Decisions

1. **Pregroup Grammar over neural classifiers**: Provides algebraic proof of injection, not just classification. Explainable, auditable.

2. **LLM-based supertagger for MVP**: Modern LLMs handle conventional indirect speech acts natively. Fine-tuned model is a v2 optimization.

3. **ag/usr domain distinction**: The critical discrimination. Agent-domain (secrets, execution, permissions) vs user-domain (content production).

4. **Voiding operators (hyp/meta/neg)**: Hypotheticals, quotation, and negation void directive force. Enables handling "What is prompt injection?" without false positives.

5. **Fail-safe defaults**: When in doubt, flag as suspicious. False positives < false negatives in security context.

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
