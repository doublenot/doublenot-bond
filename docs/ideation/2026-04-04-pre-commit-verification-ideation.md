---
date: 2026-04-04
topic: pre-commit-verification
focus: ensure commands configured in .bond/config.yml are executed before the scheduled workflow commits or opens a PR
---

# Ideation: Pre-Commit Verification For Automated PRs

## Codebase Context

The repository is a Rust CLI that bootstraps and runs a repo-local coding agent. The main workflow generator lives in [src/bond.rs](src/bond.rs), while interactive command execution lives in [src/commands.rs](src/commands.rs).

The repo already has config-driven verification commands in [.bond/config.yml](.bond/config.yml): `commands.test` and `commands.lint`. Those command groups are exposed through `/test` and `/lint` handlers in [src/commands.rs](src/commands.rs), and they already use shared RepoCommand execution logic.

The current generated GitHub Actions workflow flows from running the scheduled issue work to a combined `Commit, push, and open PR` step. That publication step is not currently guarded by configured repo verification. As a result, the agent can produce a branch and PR even if the code it just changed would fail the repo's own configured checks.

The strongest leverage point is not just adding one more shell line in YAML. The deeper seam is that workflow generation currently depends on automation settings, while verification intent lives in the broader bond settings and command groups.

No prior ideation or documented solution for this exact problem exists in `docs/ideation/` or a `docs/solutions/` tree. Existing workflow documentation is narrow and does not describe a pre-commit quality gate for scheduled automation.

## Ranked Ideas

### 1. Make Workflow Generation Consume Full Bond Settings

**Description:** Change workflow generation so it can see the repo's configured command groups, not just automation metadata. That means the generated workflow can be shaped by `commands.test` and `commands.lint`, instead of being limited to schedule/provider/model data.
**Rationale:** This is the clean architectural fix. Right now the workflow generator is structurally unable to reflect the repo's configured verification policy because it only receives automation settings. Widening that seam makes the workflow capable of enforcing repo-local quality intent instead of treating verification as unrelated CLI behavior.
**Downsides:** Slightly broader refactor than a one-line workflow tweak. It touches generator inputs and probably some tests that assume the current narrower interface.
**Confidence:** 91%
**Complexity:** Medium
**Status:** Unexplored

### 2. Add A Generated Pre-Commit Quality Gate

**Description:** Insert an explicit workflow step between `Run scheduled bond issue workflow` and `Commit, push, and open PR` that runs the configured verification commands and aborts the job on the first failure.
**Rationale:** This directly addresses the user pain. It prevents the workflow from committing, pushing, or opening a PR with code that fails the repo's own configured test and lint bar.
**Downsides:** If implemented naively in raw YAML, it can drift from the existing RepoCommand execution behavior and create two verification paths to maintain.
**Confidence:** 89%
**Complexity:** Low
**Status:** Unexplored

### 3. Restructure Publication So Staging Happens Only After Green Checks

**Description:** Move `git add -A` and all later publication logic behind successful verification, rather than only guarding `git commit` itself.
**Rationale:** This is a narrower operational refinement, but it improves failure behavior. The workflow stays easier to reason about when the job is still in a plain dirty-working-tree state until verification is known to be green.
**Downsides:** It overlaps heavily with the pre-commit gate idea and is probably best treated as a required implementation detail rather than a standalone feature.
**Confidence:** 80%
**Complexity:** Low
**Status:** Unexplored

### 4. Lock The Behavior In With Workflow-Generation Tests

**Description:** Expand the workflow-generation tests so they assert that configured checks appear before any staging, commit, push, or PR creation lines in generated `bond.yml`.
**Rationale:** This is the cheapest compounding safeguard. Once the ordering rule is encoded in tests, future workflow edits are far less likely to regress back to unsafe publication behavior.
**Downsides:** It does not solve the problem on its own; it only makes the eventual fix durable.
**Confidence:** 94%
**Complexity:** Low
**Status:** Unexplored

## Rejection Summary

| #   | Idea                                                                    | Reason Rejected                                                                                                                   |
| --- | ----------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------- |
| 1   | Add a single `/check` or `verify` command and have automation call that | Adds user-facing CLI surface area at the wrong layer; the stronger seam is workflow generation consuming existing command config. |
| 2   | Add a config-driven `before_commit` command group                       | Mostly duplicates existing `lint` and `test` groups and introduces more policy/config surface than the repo needs right now.      |
| 3   | Add a commit eligibility sentinel or success marker                     | Redundant in a single GitHub Actions job because step failure already blocks later publication steps.                             |
| 4   | Post verification failures back to the current issue                    | Useful but secondary; materially more complex and noisier than fixing the missing gate itself.                                    |
| 5   | Add a quality-gate receipt to the PR body                               | Low leverage compared with GitHub checks UI and likely to create noise or stale metadata.                                         |
| 6   | Fail closed when required command groups are absent                     | Too underspecified as a separate idea; best folded into the main gate semantics instead of introduced as new policy language.     |

## Session Log

- 2026-04-04: Initial ideation - 10 merged candidates reviewed, 4 survived
