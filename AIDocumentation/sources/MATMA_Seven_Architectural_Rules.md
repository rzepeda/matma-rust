# MATMA — Seven Architectural Rules (Authoritative Testing Methodology)

**Status:** Accepted  
**Role:** Source-of-truth architecture decision for MATMA testing, mocking and test topology.  
**Relationship:** This file is designed to accompany `MATMA_Architecture_Implementation_Blueprint_Updated.md`; the blueprint contains CLI, indexer, manifests, language-specific adapters, and the broader implementation specification.

---

## Decision: The Seven Architectural Rules (Normative)

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

## Test Tier Comparison & Test Semantics

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

## Operational Enforcement

The implementation must provide these checks:

1. `validate --check` detects invalid single-class naming/mapping and exits unsuccessfully on violations.
2. `index` refreshes read-only `.auto.yaml` dependency and mirrored-file mappings after structural changes.
3. `scaffold` generates centralized mirrored mocks, class-centric tests, and manifests when missing.
4. `impact --target <class-or-file> --level N` reads precomputed manifests to select the affected class-centric tests without reparsing source code at test-execution time.
5. Test fixtures make the **number and identity of real instances** explicit for unit (one), interaction (two), business (internal domain classes real), and regression (all participating services real) tiers.
6. The regression gateway is exercised before merge or deployment against real datasets and live/containerized services.

For the complete directory example, manifest schema, CLI command/flag matrix, implementation adapters, and engineering trade-offs, refer to the updated architecture blueprint.
