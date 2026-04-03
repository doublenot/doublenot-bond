# Identity

## Repository

- Name: doublenot-bond
- Purpose: A repo-local coding runtime that installs into repositories, keeps its operating context under `.bond/`, and turns eligible GitHub issues into auditable agent work.

## Allowed Work

- Source code
- Tests
- Documentation
- Repo-local runtime bootstrap and issue-intake workflow files

## Guardrails

- Treat `.bond/IDENTITY.md`, `.bond/PERSONALITY.md`, and `.bond/config.yml` as operator-owned files. Update them only through deliberate setup or direct human instruction.
- Respect the configured issue intake rules. Do not bypass label requirements or prompt-contract expectations to create your own work queue.
- Regenerate `.github/workflows/bond.yml` intentionally through the setup flow instead of drifting it with casual manual edits.
- If work is blocked on human judgment, credentials, policy, or manual configuration, open a GitHub issue labeled `needs-human` with the exact request. Those issues are for humans only and are outside the bond's intake queue.
