# doublenot-bond

`doublenot-bond` is a repo-local coding agent runtime. It bootstraps a `.bond/` directory inside the repository it is pointed at, keeps its identity and working state there, and uses GitHub issues as a structured task intake.

## Feature Checklist

- [x] Bootstrap a repo-local `.bond/` runtime directory
- [x] Install and re-exec through `.bond/bin/doublenot-bond`
- [x] Keep identity, personality, and journal files inside `.bond/`
- [x] Separate operator settings in `.bond/config.yml` from mutable runtime state in `.bond/state.yml`
- [x] Store scheduled automation provider, model, and cron config in `.bond/config.yml`
- [x] Support one-shot prompts and an interactive REPL
- [x] Run local slash commands without API credentials
- [x] Configure repository-specific `/test` and `/lint` workflows
- [x] Enforce permission and directory restrictions for commands and tools
- [x] Create GitHub onboarding issues and prompt-contract issue templates
- [x] Generate a scheduled GitHub Actions workflow from `.bond/config.yml`
- [x] Intake, rank, and select eligible GitHub issues
- [x] Persist current issue, last issue, parked issues, and recent issue history
- [x] Resume, reopen, park, comment on, sync, and complete issue-driven work
- [x] Record journal entries for issue lifecycle transitions
- [x] Cover bootstrap, CLI, and issue workflow behavior with automated tests
- [x] Expand README examples for non-Rust repositories and custom workflow commands
- [x] Add richer operator status summaries for active vs parked work
- [x] Add higher-level history filters beyond action-only filtering

## What It Creates

Running the binary bootstraps:

- `.bond/IDENTITY.md`
- `.bond/PERSONALITY.md`
- `.bond/JOURNAL.md`
- `.bond/config.yml`
- `.bond/bin/doublenot-bond`
- `.github/ISSUE_TEMPLATE/config.yml`
- `.github/ISSUE_TEMPLATE/bond-setup.md`
- `.github/ISSUE_TEMPLATE/bond-task.md`
- `.github/ISSUE_TEMPLATE/bond-debug.md`

The `.bond/config.yml` file is the local contract for scheduled automation, workflow commands, and issue intake rules.

The scheduled workflow file is created intentionally with `/setup workflow`:

- `.github/workflows/bond.yml`

## Basic Usage

Bootstrap the current repository:

```bash
cargo run -- --bootstrap-only
```

Run against a specific repository:

```bash
cargo run -- --repo /path/to/repo
```

Run a one-shot prompt:

```bash
cargo run -- --repo /path/to/repo --prompt "inspect the failing tests and fix the regression"
```

Use local slash commands without API credentials:

```bash
cargo run -- --repo /path/to/repo --provider ollama --prompt "/status"
```

## Install

Install the latest released binary on macOS or Linux:

```bash
curl -fsSL https://raw.githubusercontent.com/doublenot/doublenot-bond/main/install.sh | bash
```

Install the latest released binary on Windows PowerShell:

```powershell
irm https://raw.githubusercontent.com/doublenot/doublenot-bond/main/install.ps1 | iex
```

If you are using a fork, set `BOND_REPO=owner/repo` before running the installer.

## Releases

Pushing a tag like `v0.1.0` triggers `.github/workflows/release.yml`, which builds release archives for:

- Linux x86_64
- macOS x86_64
- macOS ARM64
- Windows x86_64
- Source tarball

The Linux release job installs `pkg-config` and `libssl-dev` explicitly because the published `yoagent` dependency currently links through the system OpenSSL toolchain.

Each release also publishes `doublenot-bond-checksums.txt`. The install scripts download that file and verify the archive checksum before extracting the binary, and every tagged release now includes a versioned source tarball.

The tagged release workflow now uses one aggregate publish job after artifact assembly. That job creates the GitHub Release once, attaches the platform archives plus source tarball and checksum file, validates `cargo publish --dry-run --locked`, and then publishes the crate to crates.io.

This project publishes to two distribution surfaces:

- GitHub Releases for installable binaries and release assets
- crates.io for the published Rust crate

It does not publish the Rust crate through GitHub Packages.

Before pushing a release tag, update `Cargo.toml` with:

```bash
make release-prep VERSION=0.1.1
```

That updates `package.version` and prints the next commit and tag commands. The release workflow rejects a pushed tag if `vX.Y.Z` does not match `Cargo.toml`.

For non-tag validation, run `./scripts/release-dry-run.sh`. It builds the Linux release artifact, runs `cargo publish --dry-run --locked`, creates a source tarball, and writes a checksum manifest under `target/release-dry-run/`.

For the full local validation path, run `make ci-local`.

Tagged crates.io publication requires the repository secret `CARGO_REGISTRY_TOKEN`.

## CI

`.github/workflows/ci.yml` runs on pushes to `main` and on pull requests. It installs the Linux OpenSSL build prerequisites and runs:

- `actionlint`
- `cargo fmt -- --check`
- `cargo clippy --all-targets -- -D warnings`
- `cargo test --locked`
- `./scripts/release-dry-run.sh`

## Local Actions Testing

For the closest local equivalent of the Linux CI path, run:

```bash
make ci-local
```

That target bootstraps a local `actionlint` binary under `.tools/bin/` if needed, then runs workflow linting, Rust checks, and the release dry-run.

If you only want a fast workflow syntax check, run `actionlint` directly against `.github/workflows/*.yml`.

One way to install and run it on Linux is:

```bash
curl -sSfL https://raw.githubusercontent.com/rhysd/actionlint/main/scripts/download-actionlint.bash \
  | bash -s -- latest ./bin
./bin/actionlint
```

For broader local GitHub Actions emulation, use `act`:

```bash
act pull_request -W .github/workflows/ci.yml
```

`act` is useful for the Linux CI path in this repository, but it will not faithfully cover the macOS and Windows release jobs from `.github/workflows/release.yml`.

## Git Hooks

This repository uses `cargo-husky` as a Rust-native Git hook manager.

After pulling the repository for the first time, run:

```bash
cargo test
```

That installs `.git/hooks/pre-commit` and makes each `git commit` run the configured Rust checks before the commit is accepted.

The pre-commit hook runs the same core Rust gate used locally in CI:

- `cargo fmt -- --check`
- `cargo clippy --all-targets -- -D warnings`
- `cargo test`

If you need to skip hook installation in a particular environment, set:

```bash
CARGO_HUSKY_DONT_INSTALL_HOOKS=true
```

before running Cargo commands.

## Setup Flow

The intended onboarding sequence is:

1. Bootstrap `.bond/`.
2. Customize `.bond/IDENTITY.md` and `.bond/PERSONALITY.md`.
3. Create a GitHub onboarding issue with `/setup issue`.
4. Review the generated issue templates.
5. Create or refresh the scheduled workflow with `/setup workflow`.
6. Mark setup complete with `/setup complete`.

Useful setup commands:

```text
/setup status
/setup issue
/setup workflow
/setup workflow refresh
/setup complete
/setup reset
```

## Scheduled Automation

Scheduled automation is configured in `.bond/config.yml` under `automation` and installed with `/setup workflow`.

Example:

```yaml
automation:
  schedule_cron: '0 * * * *'
  provider: anthropic
  model: claude-sonnet-4-20250514
  multiple_issues: false
  thinking_effort: medium
```

The generated `.github/workflows/bond.yml` first builds `doublenot-bond` into `.bond/bin/`, lets `--run-scheduled-issue` select the scheduled target and check out the right branch at runtime, runs the configured `commands.lint` and `commands.test` checks from `.bond/config.yml` when the scheduled run produced changes, mints a GitHub App installation token for the write path, and only then commits changes to a feature branch and opens or reuses a PR linked to the issue:

```bash
cargo build --locked --bin doublenot-bond
mkdir -p .bond/bin
cp target/debug/doublenot-bond .bond/bin/doublenot-bond
./.bond/bin/doublenot-bond --repo . --run-scheduled-issue
cargo fmt -- --check
cargo clippy --all-targets -- -D warnings
cargo test
git checkout -b "bond/issue-123-short-slug"
git add -A
git commit -m "bond: work on #123"
git push "https://x-access-token:${BOND_WRITE_TOKEN}@github.com/${GITHUB_REPOSITORY}.git" "bond/issue-123-short-slug"
gh pr create --base "${GITHUB_REF_NAME}" --head "bond/issue-123-short-slug" --title "bond: resolve #123" --body "Closes #123"
```

Scheduled runs only execute issue work when `.bond/state.yml` has `autonomous_enabled: true`, which `/setup complete` sets after you finish reviewing the generated `.bond` files. Until then, `--run-scheduled-issue` exits successfully with a skip message so the cron workflow remains a safe no-op.

`automation.multiple_issues` controls whether scheduled automation may advance to new issue work while older bond PRs are still open. The default is `false`, which keeps one bond PR in flight at a time:

- If the oldest open bond PR has `CHANGES_REQUESTED`, the scheduled run switches to that PR branch and works only on requested changes.
- If the oldest open bond PR is still open but no longer blocked by requested changes, the scheduled run stops and waits for merge or approval.
- Only when there is no open bond PR backlog does the scheduler continue the current issue or pick the next eligible issue.

When `automation.multiple_issues: true`, scheduled runs still prioritize the oldest bond PR with requested changes, but they no longer stop behind clean open PRs that are only waiting for approval or merge.

The generated workflow configures commits as `doublenot-bond[bot] <doublenot-bond[bot]@users.noreply.github.com>` and uses a GitHub App installation token for branch pushes and PR creation so the remote write actor is not `github-actions[bot]`.

When `--provider` and `--model` are omitted, `doublenot-bond` now falls back to `automation.provider` and `automation.model` from `.bond/config.yml`.

That scheduled path is no longer purely issue-driven. It first inspects open bond PRs, then either fixes the oldest PR with requested changes, pauses behind the oldest open bond PR in single-issue mode, or falls through to the existing current-issue or next-issue selection flow.

Required repository secrets depend on the configured provider:

- `anthropic` → `ANTHROPIC_API_KEY`
- `openai` → `OPENAI_API_KEY`
- `google` → `GOOGLE_API_KEY`
- `openrouter` → `OPENROUTER_API_KEY`
- `groq` → `GROQ_API_KEY`
- `deepseek` → `DEEPSEEK_API_KEY`
- `ollama` → no API-key secret is injected, but a hosted GitHub Actions runner is generally not a practical Ollama target

Scheduled write operations also require these repository secrets:

- `BOND_GITHUB_APP_ID` → the Doublenot GitHub App ID used for branch pushes and PR creation
- `BOND_GITHUB_APP_PRIVATE_KEY` → the PEM private key for that GitHub App

The workflow also uses `GITHUB_TOKEN` for issue reads, comments, closes, and any git operations performed during the run.

Generated workflows also include a per-ref concurrency group and a 30-minute timeout so overlapping cron runs do not stack up on the same branch.

Generated `.github/workflows/bond.yml` files include the configured `automation.thinking_effort` as a YAML comment near the top of the file.

## Issue Workflow

`doublenot-bond` expects GitHub task intake issues to match the prompt-contract structure and configured labels in `.bond/config.yml`.

Available issue commands:

```text
/issues list
/issues next
/issues resume
/issues select <number>
/issues reopen <number> [message]
/issues reopen-current [message]
/issues previous
/issues history [filter ...]
/issues park [message]
/issues sync
/issues current
/issues prompt
/issues start
/issues comment <message>
/issues complete [message]
/issues clear
```

Typical operator flow:

1. Use `/issues next` to accept the highest-priority eligible issue, or `/issues select <number>` to override the queue.
2. Use `/issues resume` to keep working on the current issue if one exists, otherwise restore the most recent prior issue, and only then fall back to selecting the next eligible GitHub issue.
3. Use `/issues reopen <number> [message]` to reactivate a previously closed issue, optionally post why it is being reopened, and restore it as the local current issue.
4. Use `/issues reopen-current [message]` when the last recorded issue should be reactivated without looking its number up again.
5. Use `/issues previous` to restore the most recent prior issue from local history without reopening it on GitHub.
6. Use `/issues history [filter ...]` to inspect the recent local issue stack in `.bond/state.yml`, filtered by actions like `resumed`, `parked`, `completed`, or `reopened`, or by higher-level filters like `current` and `label:bond-debug`.
7. Use `/issues park [message]` to intentionally clear the current issue while preserving it in local history as parked work.
8. Use `/issues prompt` to inspect the execution prompt that will be sent to the agent.
9. Use `/issues start` to execute that prompt through the agent runtime.
10. Use `/issues comment <message>` to post progress updates back to GitHub.
11. Use `/issues sync` if the issue changed remotely and local `.bond/state.yml` needs to be refreshed.
12. Use `/issues complete [message]` to optionally post a final comment, close the issue, and clear the local selection.

Runtime settings live in `.bond/config.yml`, while mutable runtime state now lives in `.bond/state.yml`.

Scheduled automation settings also live in `.bond/config.yml`. If you change the cron, provider, or model there, run `/setup workflow refresh` to rewrite `.github/workflows/bond.yml` intentionally.

You can set `automation.thinking_effort` to `off`, `minimal`, `low`, `medium`, or `high`. The default is `medium`, and `/status` plus `/setup status` print the configured value alongside the provider/model checks.

Recent issue retention is configurable through `.bond/config.yml` at `issues.issue_history_limit`. The default is `10`.

Issue records in `current_issue`, `last_issue`, and `issue_history` also carry lightweight metadata so local state can explain the last transition, including `last_action` and `last_action_at`.

When `/issues sync` sees that GitHub has already closed the current issue, it clears the persisted selection automatically and records that in `.bond/JOURNAL.md`.

## Workflow Commands

New `.bond/config.yml` files start with placeholder command arrays:

- `commands.test: []`
- `commands.lint: []`

The generated config includes nearby comments telling the operator to fill those arrays with repository-specific commands before using `/test`, `/lint`, or scheduled verification. Empty arrays remain intentionally incomplete, so `/test` and `/lint` will fail until real commands are configured.

Configured repositories continue to use exactly the commands declared in `.bond/config.yml`.

Example for a Node repository:

```yaml
automation:
  schedule_cron: '0 */4 * * *'
  provider: openai
  model: gpt-4.1
  thinking_effort: medium
commands:
  test:
    - program: npm
      args: ['test', '--', '--runInBand']
      description: npm test -- --runInBand
  lint:
    - program: npm
      args: ['run', 'lint']
      description: npm run lint
issues:
  eligible_labels:
    - bond-task
    - bond-bug
  priority_labels:
    - bond-bug
    - bond-task
  require_prompt_contract: true
  issue_history_limit: 10
```

Example for a Python repository:

```yaml
automation:
  schedule_cron: '30 */2 * * *'
  provider: anthropic
  model: claude-sonnet-4-20250514
  thinking_effort: medium
commands:
  test:
    - program: pytest
      args: ['-q']
      description: pytest -q
  lint:
    - program: ruff
      args: ['check', '.']
      description: ruff check .
    - program: ruff
      args: ['format', '--check', '.']
      description: ruff format --check .
issues:
  eligible_labels:
    - bond-task
    - bond-debug
  priority_labels:
    - bond-debug
    - bond-task
  require_prompt_contract: true
  issue_history_limit: 10
```

Example with custom workflow commands for a monorepo:

```yaml
automation:
  schedule_cron: '15 * * * *'
  provider: google
  model: gemini-2.5-pro
  thinking_effort: medium
commands:
  test:
    - program: pnpm
      args: ['--filter', 'web', 'test']
      description: pnpm --filter web test
    - program: pnpm
      args: ['--filter', 'api', 'test']
      description: pnpm --filter api test
  lint:
    - program: pnpm
      args: ['-r', 'lint']
      description: pnpm -r lint
issues:
  eligible_labels:
    - bond-task
    - bond-frontend
    - bond-backend
  priority_labels:
    - bond-backend
    - bond-frontend
    - bond-task
  require_prompt_contract: true
  issue_history_limit: 20
```

The key rule is that `doublenot-bond` only runs the commands you declare here. If a repository uses `make`, `just`, `tox`, `uv`, `pnpm`, or any other toolchain, encode that explicitly in `.bond/config.yml` rather than relying on placeholder defaults.

## Restrictions

The CLI supports coarse permission and directory restrictions:

```text
--allow <pattern>
--deny <pattern>
--allow-dir <path>
--deny-dir <path>
```

These restrictions apply both to slash-command execution and to wrapped tools exposed to the agent.

## Future Considerations

- Add a static Linux artifact, likely via a dedicated musl-based release target with its own OpenSSL strategy, once the extra CI and portability surface is worth the maintenance cost.
