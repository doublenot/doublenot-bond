ACTIONLINT_DIR := .tools/bin
ACTIONLINT := $(ACTIONLINT_DIR)/actionlint

.PHONY: ci-local fmt lint test release-dry-run release-prep

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

release-prep:
	@test -n "$(VERSION)" || (echo "VERSION is required, for example: make release-prep VERSION=0.1.1" && exit 1)
	./scripts/release-prep.sh $(VERSION)

ci-local: $(ACTIONLINT)
	$(ACTIONLINT)
	$(MAKE) fmt
	$(MAKE) lint
	$(MAKE) test
	$(MAKE) release-dry-run
