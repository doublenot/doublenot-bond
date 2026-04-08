# Workflow Diagrams

This document captures the current validation, release, and scheduled automation flow for `doublenot-bond`.

## Validation And Release Flow

```mermaid
flowchart TD
    A[Developer changes code] --> B{Run locally or push to GitHub?}

    subgraph Local[Local validation]
        C[make ci-local]
        C --> C1[actionlint]
        C1 --> C2[cargo fmt -- --check]
        C2 --> C3[cargo clippy --all-targets -- -D warnings]
        C3 --> C4[cargo test --locked]
        C4 --> C5[./scripts/release-dry-run.sh]
        C5 --> C6[Linux release archive + source tarball + checksums in target/release-dry-run]
    end

    subgraph CI[GitHub CI workflow]
        D[push to main or pull_request]
        D --> D1[actionlint job]
        D --> D2[test job]
        D --> D3[release-dry-run job]
        D2 --> D21[cargo fmt -- --check]
        D21 --> D22[cargo clippy --all-targets -- -D warnings]
        D22 --> D23[cargo test --locked]
        D3 --> D31[Upload dry-run artifacts]
    end

    subgraph Release[Tagged release workflow]
        E[push tag v*]
        E --> E0[verify tag matches Cargo.toml version]
        E0 --> E1[build matrix]
        E0 --> E2[source archive job]
        E1 --> E11[Linux x86_64 archive]
        E1 --> E12[macOS x86_64 archive]
        E1 --> E13[macOS ARM64 archive]
        E1 --> E14[Windows x86_64 archive]
        E2 --> E21[versioned source tarball]
        E11 --> E3[upload build artifacts]
        E12 --> E3
        E13 --> E3
        E14 --> E3
        E21 --> E3
        E3 --> E4[checksums job]
        E4 --> E5[doublenot-bond-checksums.txt artifact]
        E5 --> E6[aggregate publish job]
        E6 --> E7[create GitHub Release once]
        E6 --> E8[cargo publish dry-run]
        E8 --> E9[publish crate to crates.io]
    end

    B -->|Local| C
    B -->|Push or PR| D
    D -->|Tag push| E
```

## Notes

- `make ci-local` is the closest local equivalent to the Linux CI path.
- The release dry-run now covers both Linux packaging and `cargo publish --dry-run --locked`.
- The tagged release workflow is the only path that publishes cross-platform archives, GitHub Release assets, and the crates.io crate.
- `make release-prep VERSION=X.Y.Z` is the local step that updates `Cargo.toml` before creating the matching `vX.Y.Z` tag.
- The Rust crate is published to crates.io, not GitHub Packages.

## Scheduled Bond Workflow

```mermaid
flowchart TD
    A[Operator updates .bond/config.yml] --> B["/setup workflow"]
    B --> C[Generate .github/workflows/bond.yml]
    C --> D{GitHub trigger}

    D -->|schedule cron| E[enter per-ref concurrency group]
    D -->|workflow_dispatch| E

    E --> F[start bond job with 30-minute timeout]
    F --> G[checkout repository]
    G --> H[install pkg-config libssl-dev and Rust]
    H --> I[build doublenot-bond into .bond/bin]
    I --> J[configure git identity]
    J --> K[run .bond/bin/doublenot-bond with run-scheduled-issue]
    K --> L{Oldest bond PR needs changes?}
    L -->|Yes| M[check out PR branch and build PR-feedback prompt]
    L -->|No| N{single-issue mode blocked by open bond PR?}
    N -->|Yes| O[record merge-wait and exit 0]
    N -->|No| P{Current eligible issue already selected?}
    P -->|Yes| Q[continue current issue on its issue branch]
    P -->|No| R[select next eligible GitHub issue]
    R --> S[check out issue branch and build issue prompt]
    Q --> T[run agent against repository]
    S --> T
    M --> T
    T --> U[update .bond state, issue state, and scheduled target]
    U --> V{working tree changed?}
    V -->|Yes| V1[validate GitHub App write secrets]
    V1 --> V2[mint GitHub App installation token]
    V2 --> W[verify, commit, and push scheduled target branch]
    W --> X[open or reuse PR with Closes #issue]
    V -->|No| Y[finish without commit]
```

## Scheduled Workflow Notes

- The generated workflow builds from the checked-out repository source, stages the binary into `.bond/bin`, and runs that repo-local executable.
- New `.bond/config.yml` bootstrap output leaves `commands.test` and `commands.lint` as empty arrays with guidance comments, so operators must declare repo-specific verification commands before relying on `/test`, `/lint`, or scheduled verification.
- `automation.multiple_issues` defaults to `false`, which means the oldest open bond PR blocks new issue work until it is merged unless that PR currently has requested changes to address.
- During the scheduled run, the runtime selects and checks out the scheduled target branch itself. That target can be PR feedback work, merge-wait, or normal issue work.
- After the scheduled run, the workflow reads the persisted scheduled target metadata from `.bond/state.yml`, runs configured `commands.lint` and `commands.test` checks when there are changes to publish, validates the GitHub App secrets for the write path, mints an installation token, then stages tracked changes, commits them as `doublenot-bond[bot]`, pushes the selected branch, and opens or reuses a PR linked to the GitHub issue with `Closes #...`.
- Merge-wait scheduled runs exit successfully before verification and commit, even though they still record the pause state in `.bond` metadata.
- Cron, provider, and model are rendered from `.bond/config.yml` into `.github/workflows/bond.yml`.
- Provider API-key secret names are fixed by provider and match the runtime env lookup logic.
- Scheduled branch pushes and PR creation require `BOND_GITHUB_APP_ID` and `BOND_GITHUB_APP_PRIVATE_KEY` so the remote write actor is the configured GitHub App instead of `github-actions[bot]`.
- Generated workflows use a per-ref concurrency group and a 30-minute timeout to avoid overlapping scheduled runs on the same branch.
- `/setup workflow` preserves an existing `.github/workflows/bond.yml`; `/setup workflow refresh` overwrites it intentionally.

## Doublenot-Bond Runtime Flow

```mermaid
flowchart TD
    A[Start doublenot-bond] --> B[Parse CLI args]
    B --> C[Resolve repo root]
    C --> D[Bootstrap .bond files and issue templates]
    D --> E[Ensure repo-local runtime binary in .bond/bin]
    E --> F{Already running from repo-local binary?}
    F -->|No| G[Re-exec with --bond-runtime]
    G --> H[Load runtime context from .bond]
    F -->|Yes| H

    H --> I[Build agent config from repo, provider, model, permissions, and .bond identity]
    I --> J{Input mode?}

    J -->|--bootstrap-only| K[Exit after bootstrap]
    J -->|One-shot /command| L[Dispatch local command]
    J -->|One-shot prompt| M[Build agent and stream response]
    J -->|stdin| N{stdin starts with slash command?}
    J -->|Interactive REPL| O[Enter bond prompt loop]

    N -->|Yes| L
    N -->|No| M

    subgraph Commands[Local command path]
        L --> L1["/status or /setup"]
        L --> L2["/git, /tree, /test, /lint"]
        L --> L3["/issues" workflow]
        L3 --> L31[List, select, resume, park, sync, complete]
        L31 --> L32["Update .bond/state.yml and JOURNAL.md"]
        L31 --> L33{Need agent execution?}
        L33 -->| No | P[Return to caller or REPL]
        L33 -->| Yes: /issues start | Q[Build issue execution prompt]
    end

    subgraph Prompting[Agent execution path]
        M --> R[yoagent prompt stream]
        Q --> S[Build agent if needed]
        O --> T{Slash command or freeform prompt?}
        T -->|Slash command| L
        T -->|Freeform prompt| S
        S --> R
        R --> R1[Stream text deltas]
        R --> R2[Report tool start and end events]
        R1 --> P
        R2 --> P
    end

    K --> U[Done]
    P --> U
```

## Runtime Notes

- `.bond` is the runtime boundary for identity, personality, journal, config, state, and the repo-local executable.
- Slash commands can operate without external model credentials when they only use local repository operations.
- Issue workflows can either update local and GitHub state directly or hand off to the agent by generating an execution prompt.
