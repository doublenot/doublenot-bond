---
date: 2026-04-07
topic: bot-identity-and-release-publication
---

# Bot Identity and Release Publication

## Problem Frame

Two release-adjacent automation paths are behaving below the intended bar.

The generated bond workflow configures git author metadata as `doublenot-bond[bot]`, but its remote actions still authenticate with the repository `GITHUB_TOKEN`. That means pushes and pull request creation are performed as `github-actions[bot]`, so the visible automation identity is inconsistent with the product's intended bot persona.

Separately, the tag-triggered release workflow already attempts to upload release assets, but the current release story does not yet meet the intended distribution outcome: a reliable GitHub Release for each shipped version and a standard Rust package publication path. The desired package target is crates.io. GitHub Packages is not the publication target for the Rust crate.

## Requirements

**Automation Identity**

- R1. Scheduled bond workflow actions that mutate GitHub state must present a consistent remote identity for branch pushes and pull request creation, rather than appearing as `github-actions[bot]`.
- R2. Commit authorship and remote actor identity must align closely enough that repository history, PR activity, and audit trails clearly indicate the work came from doublenot-bond automation.
- R3. The chosen identity mechanism must be explicit in setup and documentation, including any required credentials, permissions, rotation expectations, and failure behavior when credentials are absent or invalid.

**Release Distribution**

- R4. A version tag that passes release validation must deterministically produce a visible GitHub Release for that version, with the built platform archives, source archive, and checksum asset attached.
- R5. A version tag that passes release validation must also publish the Rust crate to crates.io using the same version, with clear gating so publication only happens after the release build artifacts succeed.
- R6. Release publication failures must be surfaced clearly in workflow output so maintainers can distinguish version mismatch, credential/configuration errors, release creation failures, and crates.io publish failures.

**Operational Clarity**

- R7. Repository documentation must state the supported distribution surfaces for this project: GitHub Releases for installable binaries and crates.io for the published Rust crate.
- R8. Repository documentation must explicitly avoid implying that the Rust crate itself will appear in GitHub Packages, unless a separate non-Cargo package format is intentionally added later.

## Success Criteria

- A scheduled bond run that opens or updates a PR shows the intended doublenot-bond automation identity in the GitHub UI for the remote action, not `github-actions[bot]`.
- Pushing a valid `vX.Y.Z` tag results in a GitHub Release containing all expected assets.
- The same release flow publishes version `X.Y.Z` of `doublenot-bond` to crates.io.
- When release publication fails, the failing phase is immediately obvious from the workflow logs.

## Scope Boundaries

- No requirement to publish the Rust crate to GitHub Packages.
- No requirement to add another package format solely to populate GitHub Packages.
- No requirement to redesign the generated bond workflow beyond the identity and release/distribution concerns above.

## Key Decisions

- Distribution target: Use crates.io for the Rust package and GitHub Releases for binaries because that matches standard Rust distribution and the user confirmed GitHub Packages is not required if an equivalent GitHub-hosted artifact exists.
- Identity goal: Treat this as a real authentication problem, not a cosmetic git-config problem, because `git config user.name` alone does not change the GitHub actor used for pushes or API-created PRs.

## Dependencies / Assumptions

- crates.io publication requires a valid crates.io API token and crate ownership/configuration that allows publication from CI.
- Replacing `github-actions[bot]` with a distinct automation identity requires authenticating with credentials for that identity, such as a GitHub App installation token or a dedicated machine-user token.

## Outstanding Questions

### Resolve Before Planning

- None.

### Deferred to Planning

- [Affects R1-R3][Technical] Should the workflow authenticate as a GitHub App or as a dedicated machine user, and what are the operational tradeoffs for this repo?
- [Affects R4-R6][Technical] Should GitHub Release creation be centralized in a single post-build publish job to eliminate matrix-job race conditions and make failure states clearer?
- [Affects R5-R6][Needs research] Does the current crate metadata and ownership setup fully satisfy crates.io publication requirements from CI, or are additional manifest fields or owner changes needed?

## Next Steps

→ /ce-plan for structured implementation planning
