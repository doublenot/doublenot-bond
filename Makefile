ACTIONLINT_DIR := .tools/bin
ACTIONLINT := $(ACTIONLINT_DIR)/actionlint

.PHONY: ci-local fmt lint test release-dry-run

$(ACTIONLINT):
	mkdir -p $(ACTIONLINT_DIR)
	curl -sSfL https://raw.githubusercontent.com/rhysd/actionlint/main/scripts/download-actionlint.bash | bash -s -- latest $(ACTIONLINT_DIR)

fmt:
	cargo fmt -- --check

lint:
	cargo clippy --all-targets -- -D warnings

test:
	cargo test --locked

release-dry-run:
	./scripts/release-dry-run.sh

ci-local: $(ACTIONLINT)
	$(ACTIONLINT)
	$(MAKE) fmt
	$(MAKE) lint
	$(MAKE) test
	$(MAKE) release-dry-run
