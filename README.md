# doublenot-bond

`doublenot-bond` is a repo-local coding agent runtime. It bootstraps a `.bond/` directory inside the repository it is pointed at, keeps its identity and working state there, and uses GitHub issues as a structured task intake.

## Feature Checklist

- [x] Bootstrap a repo-local `.bond/` runtime directory
- [x] Install and re-exec through `.bond/bin/doublenot-bond`
- [x] Keep identity, personality, and journal files inside `.bond/`
- [x] Separate operator settings in `.bond/config.yml` from mutable runtime state in `.bond/state.yml`
- [x] Support one-shot prompts and an interactive REPL
- [x] Run local slash commands without API credentials
- [x] Configure repository-specific `/test` and `/lint` workflows
- [x] Enforce permission and directory restrictions for commands and tools
- [x] Create GitHub onboarding issues and prompt-contract issue templates
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

The `.bond/config.yml` file is the local contract for workflow commands and issue intake rules.

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

For non-tag validation, run `./scripts/release-dry-run.sh`. It builds the Linux release artifact, creates a source tarball, and writes a checksum manifest under `target/release-dry-run/`.

For the full local validation path, run `make ci-local`.

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

## Setup Flow

The intended onboarding sequence is:

1. Bootstrap `.bond/`.
2. Customize `.bond/IDENTITY.md` and `.bond/PERSONALITY.md`.
3. Create a GitHub onboarding issue with `/setup issue`.
4. Review the generated issue templates.
5. Mark setup complete with `/setup complete`.

Useful setup commands:

```text
/setup status
/setup issue
/setup complete
/setup reset
```

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

Recent issue retention is configurable through `.bond/config.yml` at `issues.issue_history_limit`. The default is `10`.

Issue records in `current_issue`, `last_issue`, and `issue_history` also carry lightweight metadata so local state can explain the last transition, including `last_action` and `last_action_at`.

When `/issues sync` sees that GitHub has already closed the current issue, it clears the persisted selection automatically and records that in `.bond/JOURNAL.md`.

## Workflow Commands

The default `.bond/config.yml` assumes a Rust repository and configures:

- `commands.test`: `cargo test`
- `commands.lint`: `cargo fmt -- --check` and `cargo clippy --all-targets -- -D warnings`

These can be replaced with repository-specific commands. `/test` and `/lint` only run what is explicitly configured there.

Example for a Node repository:

```yaml
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

The key rule is that `doublenot-bond` only runs the commands you declare here. If a repository uses `make`, `just`, `tox`, `uv`, `pnpm`, or any other toolchain, encode that explicitly in `.bond/config.yml` rather than assuming Rust defaults.

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
