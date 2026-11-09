# Architecture Decision Record & Implementation Specification: Multi-Tiered Agentic Testing & Mock Architecture (MATMA)

## Status
Accepted

---

## 1. Context & Core Optimization Objectives
Autonomous coding agents and localized LLM workflows operate with highest precision when verification loops are fast, deterministic, and context-bounded. MATMA optimizes five primary engineering dimensions during Test-Driven Development (TDD) and refactoring:
1. **Context-Window & Token Efficiency:** Externalizing and centralizing mock definitions, enforcing single-class files, and persisting per-class mirrored dependency manifests (`*.auto.yaml`) to disk ensures the agent only reads exact files and metadata belonging to the active task—minimizing token payloads and reducing LLM inference latency.
2. **Deterministic File & Symbol Discovery (Zero-Search Resolution):** Enforcing a predictable directory, file, and class naming convention allows orchestration tooling and agents to resolve related classes, tests, mocks, and dependency manifests mathematically, eliminating tool-call turns spent searching the filesystem (`find`, `grep`, `glob`).
3. **Disk-Materialized Reverse-Dependency Lookup ($O(1)$ File Read):** Pre-computing reverse dependencies (dependents) via a build-time indexer tool and persisting them to disk in mirrored `.auto.yaml` files allows the agent or test runner to determine downstream impact immediately via a single file read, without building or traversing an AST graph at test-execution time.
4. **Signal-to-Noise Isolation:** Strictly bounding which classes are real and which are mocked at each testing tier ensures that test failures produce unambiguous, localized diagnostic signals for the agent.
5. **Tiered Verification Scaling:** Separating class-level verification from process-level workflows and live-data regression suites allows the test runner to scale execution scope dynamically based on the persisted impact radius (`--level`).

---

## 2. Core Definition: Structural Mirroring & Atomic Class-File Isomorphism
MATMA enforces a strict, deterministic $1:1:1$ mapping (isomorphism) between the production file path, the encapsulated class symbol, and the corresponding subdirectories within the `tests/` root:

1. **Atomic Class-to-File Isomorphism (One Class Per File):** Every production file defines exactly one primary class, and the file name must deterministically match the class name (e.g., snake_case file `document_parser.<ext>` defining PascalCase class `DocumentParser`).
2. **Path Isomorphism:** Every subpackage, namespace, or directory path inside the source root (`<source_root>/<sub_path>/<class_file>.<ext>`) is replicated verbatim under respective test and manifest tiers (`tests/<tier>/mirror/<sub_path>/`).
3. **Naming & Symbol Determinism:**
   - **Production Source:** `<source_root>/<sub_path>/document_parser.<ext>` defining `DocumentParser`
   - **Unit Test Mirror:** `tests/unit/mirror/<sub_path>/test_document_parser.<ext>` defining `TestDocumentParser`
   - **Interaction Test Mirror:** `tests/interaction/mirror/<sub_path>/test_document_parser_interaction.<ext>` defining `TestDocumentParserInteraction`
   - **Mock Registry Mirror:** `tests/mocks/mirror/<sub_path>/mock_document_parser.<ext>` defining `MockDocumentParser`
   - **Persisted Dependency Mirror:** `tests/mirror/<sub_path>/document_parser.auto.yaml` containing disk-materialized reverse dependencies & mirrored test pointers.

---

## 3. Decision: The Seven Architectural Rules (Normative)

The following rules are **accepted architecture requirements**, not optional implementation suggestions. They govern source organization, Test-Driven Development (TDD), mock placement, test scope, and regression verification across MATMA language implementations.

### Rule 1 — One Class Per File with Coinciding Names
Every source, test, and mock file must encapsulate a single class whose identifier coincides deterministically with the file name, according to the target language's declared casing convention.

### Rule 2 — Isolated Unit Testing with Mocks
When performing TDD on a single class (`Class A`), instantiate `Class A` as a real object and use mock classes for **all external interactions and collaborators**, completely isolating the unit under test.

### Rule 3 — Centralized Mirrored Mock Registries
Mocks are centralized in their own source-mirrored structure (`tests/mocks/mirror/...`). Tests import reusable mock classes from this registry instead of defining inline mocks. This minimizes duplicated mock definitions, reduces agent context-window consumption, and establishes a single source of truth for each test double.

### Rule 4 — Strict Pairwise Interaction Isolation
When testing the interaction of two collaborating classes (`Class A` calling into `Class B`), instantiate **both classes as real objects**. Replace **all other downstream collaborators** needed by either class with their centralized mirrored mocks. The purpose is to verify the contract between the selected real pair, not to run an unrestricted integration graph.

### Rule 5 — Service-Level Boundary Mocking for Workflows
For multi-step workflows and business-logic interactions (`tests/business/`), execute internal domain models and business classes normally. Mock underlying **external or infrastructure services at the service boundary** (for example: databases, LLM providers, vector stores, HTTP clients). Keep workflow tests process-oriented rather than source-path-mirrored.

### Rule 6 — Integrated Regression Gateways
Maintain an end-to-end regression suite in `tests/regression/` using **real datasets and real/live/containerized services** to verify complete system integrity before deployment or merge. Regression tests do **not** substitute mocks for participating services.

### Rule 7 — Bifurcated Directory Topology (Mirrored vs. Process-Oriented)
- **Class-centric tiers:** `tests/unit/mirror/`, `tests/interaction/mirror/`, `tests/mocks/mirror/`, and `tests/mirror/**/*.auto.yaml` strictly mirror production source paths and deterministic class/file names. Manifests persist source-relative mapping and reverse dependencies.
- **Process-centric tiers:** `tests/business/` and `tests/regression/` are **not mirrored** to source paths. They are organized by business capability, workflow, feature domain, user journey, or end-to-end pipeline.

> **Precedence:** The seven rules are the governing test-methodology decision. The directory tree, indexer, CLI and language adapters below implement these rules; they must not weaken the specified real-vs-mock boundaries.

---

## 4. Architectural Tier Comparison & Test Semantics

| Tier | Base path | Directory organization | File/class convention | Scope | Real objects | Mock boundary |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- |
| Source | `<source_root>/` | Domain/source package tree | 1 production class per file | Production behavior | N/A | N/A |
| Dependency manifests | `tests/mirror/` | **Mirrored** 1:1 | 1 generated `.auto.yaml` per source class | Dependency metadata, inverse graph, file pointers | N/A | N/A |
| Mock registry | `tests/mocks/mirror/` | **Mirrored** 1:1 | 1 `Mock<Class>` per mock file | Shared test doubles | N/A | Public interface of corresponding source class |
| Unit | `tests/unit/mirror/` | **Mirrored** 1:1 | 1 `Test<Class>` per test file | One class (`A`) | `A` | **Every collaborator of `A`** |
| Interaction | `tests/interaction/mirror/` | **Mirrored** by target class | 1 `Test<Class>Interaction` per test file | One selected collaborating pair (`A` + `B`) | `A` and `B` | **Every other downstream collaborator of either** |
| Business | `tests/business/` | **Process-oriented** (non-mirrored) | 1 workflow test class per test file | Business workflow spanning multiple internal classes | Internal domain and workflow classes | **External/infrastructure services only, at boundaries** |
| Regression | `tests/regression/` | **Process-oriented** (non-mirrored) | 1 regression test class per test file | Full system, data pipeline, end-to-end journey | Full application and real/live/containerized services | **None**; use real datasets and services |

**Structural mapping example:** `core/document_parser.<ext>` defining `DocumentParser` deterministically maps to `tests/unit/mirror/core/test_document_parser.<ext>` (`TestDocumentParser`), `tests/interaction/mirror/core/test_document_parser_interaction.<ext>` (`TestDocumentParserInteraction`), `tests/mocks/mirror/core/mock_document_parser.<ext>` (`MockDocumentParser`), and `tests/mirror/core/document_parser.auto.yaml` (metadata for `DocumentParser`). The business and regression paths remain independent of `core/`.

### Verification intent by level

- `--level 0`: read the target class's manifest and execute only its mirrored unit test.
- `--level 1`: include target interaction verification and unit/interaction tests associated with direct dependents (one reverse-dependency hop), using precomputed manifests.
- `--level 2+`: traverse persisted reverse dependencies through the requested number of hops to resolve further affected class-centric tests.
- Business tests validate workflows with infrastructure boundaries mocked; regression tests validate full end-to-end behavior with real services. They are **separate verification tiers**, not directories reconstructed from class names or solely implied by `--level`.

---

## 5. Tooling Requirements & Hosting Strategy
To keep core application repositories pristine, unobtrusive, and free of clutter:
* **Built on GitHub:** Each language implementation resides in a dedicated standalone repository (`matma-python`, `matma-php`, `matma-rs`).
* **Zero Workspace Clutter:** Tools are imported cleanly via native package managers (`pip`, Composer `vendor/`, or Cargo dependencies/caches) or managed inside git-ignored local tooling directories (`.tools/` or `bin/`).
* **Agent-Friendly CLI:** Every implementation exposes a standardized CLI interface enabling agents to trigger indexing, scaffolding, impact querying, and validation in single command-line turns.

---

## 6. Standardized CLI Command Specification & Flag Matrix

### Commands
1. **`index`**: Parses ASTs, builds dependency graph, inverts edges, and serializes `.auto.yaml` manifests to disk.
2. **`scaffold`**: Automatically provisions missing unit tests, interaction tests, centralized mocks, and manifests for new source classes.
3. **`impact`**: Queries disk-persisted reverse dependencies at a specified depth (`--level N`) for targeted test execution.
4. **`validate`**: Audits codebase for structural invariants (one-class-per-file and naming match rules).

### Flag Parameters Matrix
| Flag / Option | Short | Default | Description |
| :--- | :--- | :--- | :--- |
| `--source-root` | `-s` | `src/` (or `app/` for PHP) | Root directory containing production source code. |
| `--tests-root` | `-t` | `tests/` | Root directory containing unit, interaction, mock, and mirror tiers. |
| `--scaffold` | `-S` | `false` | Automatically generates missing mirrored test files, mock stubs, and manifests for new classes. |
| `--level` | `-l` | `0` | Impact radius depth for test execution (`0` = unit test only; `1` = interaction tests; `2+` = transitive dependents). |
| `--target` | `-f` | `None` | Specifies a single source file or class to analyze or query. |
| `--check` | `-c` | `false` | Strict audit mode; exits with non-zero code if structural invariants are violated. |
| `--format` | `-o` | `text` | Output serialization format (`text`, `json`, `yaml`). |

---

## 7. Directory Topology & Mapping Matrix

```text
<source_root>/                                             # Production Source Root
├── core/
│   ├── document_parser.<ext>                              # Defines ONLY: class DocumentParser
│   └── semantic_chunker.<ext>                             # Defines ONLY: class SemanticChunker
└── services/
    └── ingestion_service.<ext>                            # Defines ONLY: class IngestionService

tests/                                                     # Test Suite Root
├── mirror/                                                # [MIRRORED] Disk-Persisted Reverse-Dependency Manifests
│   ├── core/
│   │   ├── document_parser.auto.yaml                      # Saved reverse dependencies & mirror links
│   │   └── semantic_chunker.auto.yaml
│   └── services/
│       └── ingestion_service.auto.yaml
│
├── mocks/                                                 # [MIRRORED] Centralized Test Doubles
│   └── mirror/
│       ├── core/
│       │   ├── mock_document_parser.<ext>                 # Defines ONLY: class MockDocumentParser
│       │   └── mock_semantic_chunker.<ext>
│       └── services/
│           └── mock_ingestion_service.<ext>
│
├── unit/                                                  # [MIRRORED] Single-Class Isolation
│   └── mirror/
│       ├── core/
│       │   ├── test_document_parser.<ext>                 # Defines ONLY: class TestDocumentParser
│       │   └── test_semantic_chunker.<ext>
│       └── services/
│           └── test_ingestion_service.<ext>
│
├── interaction/                                           # [MIRRORED] Pairwise Class Interaction
│   └── mirror/
│       └── core/
│           └── test_document_parser_interaction.<ext>     # Defines ONLY: class TestDocumentParserInteraction
│
├── business/                                              # [NON-MIRRORED] Workflow / Process Tests
│   └── test_document_ingestion_workflow.<ext>
│
└── regression/                                            # [NON-MIRRORED] E2E Data & Service Tests
    └── test_end_to_end_pipeline.<ext>
```

---

## 8. Example Disk-Persisted Manifest Schema (`document_parser.auto.yaml`)

```yaml
# AUTO-GENERATED BY MATMA AST INDEXER TOOL - DO NOT EDIT MANUALLY
class_name: "DocumentParser"
source_file: "<source_root>/core/document_parser.<ext>"

mirrors:
  unit_test: "tests/unit/mirror/core/test_document_parser.<ext>"
  interaction_test: "tests/interaction/mirror/core/test_document_parser_interaction.<ext>"
  mock_file: "tests/mocks/mirror/core/mock_document_parser.<ext>"
  mock_class: "MockDocumentParser"

dependencies:
  - class_name: "TextNormalizer"
    source_file: "<source_root>/utils/text_normalizer.<ext>"
    mock_file: "tests/mocks/mirror/utils/mock_text_normalizer.<ext>"
    mock_class: "MockTextNormalizer"

dependents:
  level_1:
    - class_name: "SemanticChunker"
      source_file: "<source_root>/core/semantic_chunker.<ext>"
      unit_test: "tests/unit/mirror/core/test_semantic_chunker.<ext>"
      interaction_test: "tests/interaction/mirror/core/test_semantic_chunker_interaction.<ext>"
  level_2:
    - class_name: "RagController"
      source_file: "<source_root>/api/rag_controller.<ext>"
      unit_test: "tests/unit/mirror/api/test_rag_controller.<ext>"
      interaction_test: "tests/interaction/mirror/api/test_rag_controller_interaction.<ext>"
```

---

## 9. Language-Specific Implementation Blueprint

### 1. Python (`matma-py`)
* **AST Engine:** Built-in `ast` module (`ast.parse()`, `ast.ClassDef`, `ast.Import`, `ast.ImportFrom`).
* **Serialization:** `PyYAML` for `.auto.yaml` generation.
* **CLI Entrypoint:** Configured in `pyproject.toml` via `[project.scripts]` (`matma-indexer = "matma.cli:main"`).

### 2. PHP-Laravel (`matma-php`)
* **AST Engine:** `nikic/php-parser` for inspecting PHP statements (`Stmt\Class_`, `Stmt\Use_`).
* **Serialization:** Symfony YAML Component (native to Laravel).
* **CLI Entrypoint:** Custom Artisan Command (`php artisan matma:index`).

### 3. Rust (`matma-rs`)
* **AST Engine:** `syn` and `proc-macro2` crates for parsing Rust items (`Item::Struct`, `Item::Enum`, `Item::Use`).
* **Serialization:** `serde` and `serde_yaml`.
* **CLI Entrypoint:** Standalone binary CLI built with `clap` (`matma-indexer`).

---

## 10. Static Indexer Lifecycle, Consequences & Operational Requirements

The MATMA static reverse-dependency and mirrored-manifest indexer is an **ahead-of-time** build/CLI tool (referred to in the original methodology as `build_tests_yaml`). Its required responsibilities are:

1. **Validate the structural invariant:** parse source syntax trees and check single-class files plus deterministic class/file names.
2. **Compute and persist dependencies:** scan source files; extract outbound imports/references to source classes; build and invert the dependency graph; write a **read-only generated** manifest for each source class at `tests/mirror/<sub_path>/<class_name>.auto.yaml`. The manifest records source identity, unit/interaction/mock pointers, outbound dependencies with mock pointers, and precomputed inbound dependents grouped by impact depth.
3. **Scaffold new classes:** generate missing mirrored unit-test, interaction-test, mock, and manifest files when requested (`scaffold` / `--scaffold`).
4. **Resolve impact without runtime AST parsing:** use the persisted manifests for `impact --target ... --level N`. For the direct target, manifest discovery is a constant-number file lookup; levels 2+ traverse multiple precomputed manifest files rather than reparsing the entire source AST. The exact traversal cost depends on the number of reachable classes.

### Consequences and acceptance checks

- **Token-bounded context:** agents can load one target manifest plus only relevant sources, tests, mocks, and affected dependents.
- **Deterministic discovery:** agents should derive mirrored paths from source paths rather than searching the repository to locate companions.
- **Re-index on structural edits:** any added/removed imports, source classes, or changes to class identity require regenerating the `.auto.yaml` manifests before using impact data. Generated manifests must not be hand-edited.
- **Mock/interface synchronization:** public interface changes require corresponding updates to centralized mocks and associated pairwise interaction tests.
- **Test integrity:** unit tests must not instantiate real collaborators; pairwise tests must not accidentally include a third real collaborator; workflow tests must not mock their internal domain behavior; regression tests must not replace participating services with mocks.
- **Pre-merge/deployment gateway:** execute the dedicated regression suite against real datasets and live/containerized infrastructure before merging or deploying, as required by Rule 6.

### Implementation notes that require language-specific interpretation

These notes do not alter the accepted seven-rule decision. Languages without a universal `class` construct (e.g. Rust) must define a documented mapping from the single-primary-type and naming invariant to their native constructs (such as a type plus its `impl` blocks). Any decision to allow multiple primary types per source file would change Rule 1 and therefore requires an explicit new architecture decision. Import/AST analysis must resolve actual source-class dependencies; a raw import line alone may not identify a unique class in every language.
