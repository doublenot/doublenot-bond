---
title: feat: Generic workflow command defaults
type: feat
status: completed
date: 2026-04-05
origin: docs/brainstorms/2026-04-04-generic-workflow-command-defaults-requirements.md
---

# feat: Generic workflow command defaults

## Overview

Change the initial `.bond/config.yml` bootstrap output for newly created repos so `commands.test` and `commands.lint` use empty placeholder arrays with nearby guidance comments instead of Rust-specific Cargo commands. This plan is intentionally about bootstrap defaults, not runtime fallback behavior for existing repos. Existing repositories stay stable by avoiding silent config rewrites, and empty command groups continue to represent an unconfigured state rather than a passing verification state.

## Problem Frame

`src/bond.rs` currently makes `doublenot-bond` feel Rust-specific by seeding new configs with `cargo test`, `cargo fmt -- --check`, and `cargo clippy --all-targets -- -D warnings`. The origin requirements document establishes that this bootstrap behavior should become repository-agnostic while keeping operator-owned command settings untouched in existing repos and during config maintenance flows. In this plan, "defaults" means the content written when `.bond/config.yml` is first created; it does not mean a runnable fallback command set for existing repositories. The current codebase also exposes an implementation constraint: `save_bond_settings` is a `serde_yaml::to_string` round trip, so any solution that depends on preserved YAML comments cannot rely on the generic serializer path. See origin: `docs/brainstorms/2026-04-04-generic-workflow-command-defaults-requirements.md`.

## Requirements Trace

- R1. New `.bond/config.yml` files must not bootstrap repo-specific test or lint commands.
- R2. Default `commands.test` and `commands.lint` must be empty arrays.
- R3. Bootstrap output must include nearby operator guidance comments for those arrays.
- R4. Existing customized commands must remain unchanged during config maintenance flows.
- R5. Placeholder defaults apply only to missing or newly created config content.
- R6. Existing repos with legacy Cargo defaults are not auto-migrated.
- R7. Generated workflow and setup flows continue to treat commands as operator-owned settings.
- R8. Tests and documentation must explain the new placeholder behavior.
- R9. Empty command arrays must continue to behave as unconfigured, not as successful verification.

## Scope Boundaries

- No automatic migration of existing `.bond/config.yml` files.
- No new command groups beyond `commands.test` and `commands.lint`.
- No change to how configured commands execute once present.
- No attempt to preserve arbitrary user formatting across full config rewrites; the plan avoids those rewrites instead.

## Context & Research

### Relevant Code and Patterns

- `src/bond.rs` owns bootstrap, config persistence, and workflow generation.
- `BondPaths::bootstrap_bond_files` only writes `.bond/config.yml` when the file is missing.
- `BondPaths::save_bond_settings` currently serializes `BondSettings` through `serde_yaml::to_string`, which is unsuitable for comment-preserving writes.
- `default_issue_template_config_contents` and `default_bond_workflow_contents` show the existing pattern for template-managed YAML emitted as strings rather than serializer round trips.
- `src/commands.rs` routes `/setup workflow`, `/setup workflow refresh`, `/setup reset`, `/test`, and `/lint`.
- `run_repo_commands` already enforces the current incomplete-config behavior by failing on empty command lists with `No commands configured for this workflow in .bond/config.yml.`
- `tests/bootstrap.rs` verifies bootstrap output and parses `.bond/config.yml` as YAML.
- `tests/commands.rs` already covers workflow refresh behavior and empty-command failure semantics.

### Institutional Learnings

- No prior `docs/solutions/` entry or repo memory note defines a pattern for placeholder command defaults or config rewrite preservation.

### External References

- External research was intentionally skipped. The repo already has strong local patterns for template-generated YAML and the change is repo-contract work rather than framework-specific integration work.

## Key Technical Decisions

- Use a bootstrap-specific config renderer instead of changing the generic serializer path: this is the only practical way to emit guidance comments while keeping runtime reads on `BondSettings`.
- Keep one authoritative config schema: the bootstrap renderer should derive its structure from the same `BondSettings` defaults or a shared rendering helper so comment-aware bootstrap output does not become a drifting second source of truth.
- Preserve operator-owned commands by narrowing write scope, not by adding implicit migration logic: bootstrap writes the config once when it is missing, while workflow refresh and reset continue to avoid rewriting `.bond/config.yml`.
- Keep execution semantics unchanged for empty arrays: the existing `/test`, `/lint`, and workflow verification behavior already models empty command groups as unconfigured, which satisfies R9 without inventing a new sentinel state.
- Treat legacy Cargo defaults as historical data, not as template debt: existing repos remain untouched unless an operator edits the file directly.
- Ship one effective command-normalization contract: omitted or explicit-null command groups should normalize to empty vectors on load, and those empty vectors should still fail execution as unconfigured.

## Open Questions

### Resolved During Planning

- How should comments be emitted without breaking normal config persistence? Use a bootstrap-only string/template writer for initial config creation and leave serializer-based settings persistence comment-free.
- Which current code paths qualify as config maintenance flows? In the current repo, `/setup workflow refresh` rewrites `.github/workflows/bond.yml` only, and `/setup reset` only updates `.bond/state.yml`; neither should start round-tripping `.bond/config.yml` as part of this feature.
- How should empty placeholder arrays behave at runtime? Keep the existing failure behavior from `run_repo_commands` so unconfigured commands are visible rather than silently passing.
- How should missing and `null` command groups behave? Normalize them to the same effective empty-vector state as `[]`, while preserving the visible execution failure for unconfigured commands.

### Deferred to Implementation

- Whether a future explicit config reinitialize command should share the bootstrap template writer or expose a separate, operator-confirmed reset contract. The current repo does not have that flow yet.

## High-Level Technical Design

> _This illustrates the intended approach and is directional guidance for review, not implementation specification. The implementing agent should treat it as context, not code to reproduce._

```mermaid
flowchart TB
    A[Bootstrap with missing .bond/config.yml] --> B[Render config template with empty test/lint arrays and guidance comments]
    B --> C[Write file once]
    D[Existing .bond/config.yml] --> E[Load with BondSettings serde path]
    E --> F[/setup workflow refresh or /setup reset]
    F --> G[Do not rewrite commands section]
    E --> H[/test, /lint, workflow verification]
    H --> I{Command array empty?}
    I -->|Yes| J[Fail as unconfigured]
    I -->|No| K[Run configured commands]
```

## Implementation Units

- [x] **Unit 1: Add bootstrap-only config rendering**

**Goal:** Replace repo-specific command defaults in newly created `.bond/config.yml` files with empty arrays and nearby guidance comments.

**Requirements:** R1, R2, R3, R6

**Dependencies:** None

**Files:**

- Modify: `src/bond.rs`
- Test: `tests/bootstrap.rs`

**Approach:**

- Introduce a dedicated helper in `src/bond.rs` that renders default config YAML as a string for bootstrap creation only.
- Keep the rendered structure aligned with the existing `BondSettings` schema through a shared helper or shared default source so `load_bond_settings` can continue using `serde_yaml` without template drift.
- Limit the new renderer to the `if !self.config_file.exists()` path in `BondPaths::bootstrap_bond_files` so existing repos are not rewritten.
- Leave `default_workflow_commands()` available only if the implementation still needs a programmatic default for in-memory structures; otherwise replace that usage with an empty-array default consistent with the bootstrap template.

**Patterns to follow:**

- String-rendered YAML helpers in `src/bond.rs`, especially `default_issue_template_config_contents` and `default_bond_workflow_contents`
- Missing-file bootstrap guard in `BondPaths::bootstrap_bond_files`

**Test scenarios:**

- Happy path: bootstrap-only on an empty repo creates `.bond/config.yml` where `commands.test` and `commands.lint` parse as empty arrays.
- Happy path: bootstrap-only writes operator guidance comments adjacent to the `commands` section so a human can see the config is intentionally incomplete.
- Edge case: parsing the bootstrapped config through `serde_yaml::from_str::<Value>` still succeeds even with the added comments.
- Integration: a repo bootstrapped with the new config can still be loaded into runtime state without changing `BondSettings` shape.

**Verification:**

- A fresh bootstrap produces a valid `.bond/config.yml` with empty arrays and guidance comments, and no Rust-specific command defaults appear in that file.

- [x] **Unit 2: Lock preservation boundaries for config maintenance flows**

**Goal:** Ensure the feature does not introduce accidental rewrites of operator-owned command settings during workflow refresh or reset-like setup flows.

**Requirements:** R4, R5, R6, R7

**Dependencies:** Unit 1

**Files:**

- Modify: `src/bond.rs`
- Modify: `src/commands.rs`
- Test: `tests/commands.rs`

**Approach:**

- Audit any setup path that could write `.bond/config.yml` and keep config creation limited to missing-file bootstrap behavior.
- Avoid repurposing `save_bond_settings` inside workflow refresh or reset flows as part of this feature.
- If naming or helper boundaries are currently too generic, rename or document them so future changes do not mistake workflow refresh for a safe place to normalize config content.
- Add regression tests that prove workflow refresh still rewrites only `.github/workflows/bond.yml` and leaves custom command arrays intact.

**Patterns to follow:**

- Existing `/setup workflow refresh` behavior in `src/commands.rs`
- Existing workflow-refresh assertions in `tests/commands.rs`
- Operator-owned file guidance in `.bond` bootstrap text within `src/bond.rs`

**Test scenarios:**

- Happy path: `/setup workflow refresh` with a customized `.bond/config.yml` rewrites the workflow file while leaving the config file content unchanged.
- Happy path: `/setup reset` continues to update setup state without modifying configured command arrays.
- Edge case: an existing repo whose config still contains legacy Cargo defaults is not normalized to placeholders during workflow refresh.
- Integration: refreshing the workflow after customizing commands still renders those commands into `.github/workflows/bond.yml` exactly as configured.

**Verification:**

- Maintenance flows remain read-only with respect to `.bond/config.yml` command content unless the config file is missing and bootstrap is creating it for the first time.

- [x] **Unit 3: Characterize and harden command deserialization edges**

**Goal:** Define and implement safe behavior for missing, partial, and explicit-null command keys without changing the visible incomplete-config semantics.

**Requirements:** R5, R7, R9

**Dependencies:** Unit 1

**Files:**

- Modify: `src/bond.rs`
- Test: `tests/commands.rs`

**Approach:**

- Start with characterization tests around `BondSettings` loading for configs that omit `commands`, omit only one command group, or supply explicit `null` values.
- Normalize omitted and explicit-null command groups to empty vectors during load.
- Preserve the current runtime contract that the resulting empty command groups fail as unconfigured.
- Keep partial command configs operator-owned: a populated `test` group must survive while an omitted or null `lint` group normalizes only its own side.

**Execution note:** Add characterization coverage before changing deserialization behavior.

**Patterns to follow:**

- `#[serde(default)]` usage on `WorkflowCommands` and `BondSettings`
- Existing empty-command failure path in `run_repo_commands`

**Test scenarios:**

- Happy path: config with `commands` omitted loads with empty `test` and `lint` vectors and `/lint` still fails as unconfigured.
- Edge case: config with only `commands.test` populated preserves that test command while defaulting `commands.lint` to an empty list.
- Edge case: config with `commands: null`, `test: null`, or `lint: null` normalizes those groups to empty vectors without erasing sibling command groups.
- Error path: `/test` and `/lint` continue to emit `No commands configured for this workflow in .bond/config.yml.` when the effective command list is empty.

**Verification:**

- The plan’s normalization rules are covered by tests, and no path converts empty placeholders into a successful verification state.

- [x] **Unit 4: Update operator-facing docs and examples**

**Goal:** Align documentation with the new repository-agnostic bootstrap behavior and the expectation that operators must fill in repo-specific commands.

**Requirements:** R8, R9

**Dependencies:** Units 1-3

**Files:**

- Modify: `README.md`
- Modify: `docs/workflows.md`

**Approach:**

- Replace language that says the default config assumes a Rust repository.
- Explain that bootstrap now emits placeholders and that empty arrays intentionally fail until the operator adds real commands.
- Keep the Node, Python, and monorepo examples as examples of explicit operator configuration rather than inferred defaults.

**Patterns to follow:**

- Existing `README.md` sections for workflow commands and scheduled workflow behavior
- Existing `docs/workflows.md` notes about operator updates to `.bond/config.yml`

**Test scenarios:**

- Test expectation: none -- documentation-only unit.

**Verification:**

- A maintainer reading the docs can tell that new repos start with placeholder command arrays and must opt into concrete test and lint commands.

## System-Wide Impact

- **Interaction graph:** Bootstrap writes `.bond/config.yml`; runtime load reads it into `BondSettings`; `/test`, `/lint`, and generated workflow verification consume the resulting command arrays.
- **Error propagation:** Empty command arrays continue to fail through `run_repo_commands`, surfacing incomplete configuration instead of masking it.
- **State lifecycle risks:** The main risk is accidental config churn from serializer-based rewrites. The plan avoids that by keeping comment-bearing output on a bootstrap-only write path and by treating any future config-reinitialize flow as separate design work.
- **API surface parity:** `.bond/config.yml` remains the single operator contract for command configuration across interactive commands and scheduled workflow generation.
- **Integration coverage:** End-to-end tests should continue proving that customized command arrays flow from config into generated workflow content without being normalized away.
- **Unchanged invariants:** Existing repos keep their current command arrays, including historical Cargo defaults, and configured commands still execute exactly as declared.

## Risks & Dependencies

| Risk                                                                          | Mitigation                                                                                                                                       |
| ----------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------ |
| Bootstrap comments are lost if the implementation reuses `save_bond_settings` | Keep comment-bearing output on a dedicated bootstrap renderer and do not rely on serializer round trips for initial file creation                |
| Bootstrap template drifts from `BondSettings` defaults over time              | Derive bootstrap rendering from the same default source or a shared helper instead of maintaining a fully separate handwritten schema            |
| Preservation rules become ambiguous in future setup flows                     | Make the bootstrap-only boundary explicit in helper naming, tests, and docs so future reset or reinitialize features have to opt in deliberately |
| `null` handling changes config compatibility unexpectedly                     | Add characterization tests first, then implement the documented normalization contract for omitted and explicit-null command groups              |
| Docs drift from actual bootstrap output                                       | Update both `README.md` and `docs/workflows.md` in the same change set as the bootstrap renderer                                                 |

## Documentation / Operational Notes

- No rollout or migration step is required because the feature applies only to newly created config files.
- This plan should leave the checked-in `.bond/config.yml` in this repository untouched.
- Guidance comments are treated as a bootstrap affordance, not a persistence guarantee across future explicit config rewrites.
- If a future config reinitialize command is added, it should be planned separately instead of being folded implicitly into this work.

## Sources & References

- Origin document: `docs/brainstorms/2026-04-04-generic-workflow-command-defaults-requirements.md`
- Related code: `src/bond.rs`, `src/commands.rs`
- Related tests: `tests/bootstrap.rs`, `tests/commands.rs`
- Related docs: `README.md`, `docs/workflows.md`
