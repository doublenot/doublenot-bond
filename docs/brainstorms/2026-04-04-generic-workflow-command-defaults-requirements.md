---
date: 2026-04-04
topic: generic-workflow-command-defaults
---

# Generic Workflow Command Defaults

## Problem Frame

`src/bond.rs` currently bootstraps `commands.test` and `commands.lint` with Rust-specific Cargo commands through `default_workflow_commands()`. That makes the default `.bond/config.yml` feel repo-specific instead of repository-agnostic, and it creates the wrong starting point for non-Rust repositories or repos that use different quality gates.

The goal is to make the initial command configuration clearly incomplete by default while preserving any commands an operator has already customized. This should reduce accidental coupling to this repository's current Rust setup without causing config churn for existing users.

For this document, `config maintenance flows` means non-interactive setup operations that regenerate or refresh template-managed repository artifacts without being a direct human edit to `.bond/config.yml`. That includes workflow refresh and any future explicit reset or reinitialize path that could otherwise rewrite default config content.

## Requirements

**Bootstrap Defaults**

- R1. New `.bond/config.yml` files must not be bootstrapped with repository-specific test or lint commands.
- R2. Default `commands.test` and `commands.lint` entries must be represented as empty arrays.
- R3. The default config content must include a nearby comment that tells the operator those command lists need to be filled in for the target repository.

**Preservation Rules**

- R4. Existing customized `commands` values must remain unchanged during config maintenance flows.
- R5. Placeholder defaults are only for missing or newly created config content; they must not replace already populated command lists.
- R6. Repositories whose `.bond/config.yml` already contains the current Cargo-based command defaults from configs created before this change must be left untouched unless an operator deliberately edits them.

**User Expectations**

- R7. The generated workflow and related config maintenance flows must continue to treat configured commands as operator-owned repo settings rather than silently normalizing them back to template defaults.
- R8. Documentation and tests must make it clear that generic command placeholders are intentional and that operators are expected to add repo-appropriate test and lint commands.
- R9. If an operator invokes test or lint behavior before configuring those command lists, the system must continue the current incomplete-config behavior rather than silently treating empty arrays as success.

## Success Criteria

- New bootstrap output no longer implies this tool is Rust-specific through default test and lint commands.
- Existing repositories do not lose or have their customized `commands` section rewritten during workflow refresh or similar maintenance flows.
- A maintainer reading the generated default config can immediately see that test and lint commands still need to be configured.
- Empty placeholder command arrays do not blur into a successful verification state before the operator configures them.

## Scope Boundaries

- No automatic migration of existing `.bond/config.yml` files.
- No new command groups beyond the existing `test` and `lint` sections.
- No changes to how already configured commands are executed.

## Key Decisions

- Empty arrays with comments are the default shape: this keeps the schema stable while making the missing configuration explicit.
- Preservation beats normalization for existing configs: once commands are customized, config maintenance flows should not silently rewrite them.
- Existing Cargo defaults are not retroactively replaced: the change applies to future bootstrap behavior, not to already initialized repositories.
- Empty placeholder arrays remain intentionally incomplete: until operators add real commands, test/lint execution continues to behave like an unconfigured workflow rather than a passing one.

## Dependencies / Assumptions

- Assumes the config creation or update path can render the placeholder guidance in a way that remains readable to operators, even if YAML comment preservation requires a template-aware write path rather than a serializer round trip.

## Outstanding Questions

### Deferred to Planning

- [Affects R3][Technical] How comments for unset command lists should be generated or preserved given the current config serialization approach.
- [Affects R4][Technical] Which concrete code paths qualify as config maintenance flows today, and whether any additional guards are needed beyond current workflow refresh behavior.
- [Affects R5][Technical] How missing, `null`, or partially populated `commands` keys should be normalized when reading and writing `.bond/config.yml`.

## Next Steps

→ /ce-plan for structured implementation planning
