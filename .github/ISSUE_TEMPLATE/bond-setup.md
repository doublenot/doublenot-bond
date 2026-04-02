---
name: Bond Setup
about: Configure the .bond runtime for this repository
title: "Bond setup: configure doublenot-bond"
labels: bond-setup
assignees: ''
---

## Inputs

- Review [.bond/IDENTITY.md](../.bond/IDENTITY.md) and replace placeholders with repository-specific intent and guardrails.
- Review [.bond/PERSONALITY.md](../.bond/PERSONALITY.md) and set the collaboration style this bond should follow.
- Confirm whether autonomous work should remain disabled until onboarding is reviewed.

## Expected Output

- `.bond` files reflect the repository's real scope and constraints.
- Prompt-contract issue templates are available under `.github/ISSUE_TEMPLATE/`.
- The repository is ready for `/setup complete` after human review.

## Constraints

- Do not enable autonomous execution until the repository owner has reviewed the `.bond` files.
- Keep repository-specific rules in `.bond`, not in ad hoc chat context.
- Preserve existing repo conventions and contribution rules.

## Edge Cases

- If the repository is not hosted on GitHub, document that blocker before marking setup complete.
- If existing issue templates already exist, reconcile them instead of deleting them blindly.
- If `.bond` conflicts with project policy, record the alternative onboarding path here.

## Acceptance Criteria

- [ ] `.bond/IDENTITY.md` is customized for this repository.
- [ ] `.bond/PERSONALITY.md` is customized for this repository.
- [ ] `.bond/JOURNAL.md` has an initial repository-specific entry.
- [ ] The repository owner agrees the bond can begin work.
- [ ] `doublenot-bond --prompt "/setup complete"` is the next step after review.
