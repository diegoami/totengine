.PHONY: test rust fmt clippy evidence-regen evidence-check

CARGO ?= cargo

rust:
	$(CARGO) test --workspace

fmt:
	$(CARGO) fmt --all -- --check

clippy:
	$(CARGO) clippy --workspace --all-targets -- -D warnings

# Full local test suite: Rust + Python probe compile + golden corpus.
# The golden part self-skips when CIVTOT_HOME is unset (e.g. CI).
test: rust
	python3 -m py_compile tools/totctl.py
	bash tools/golden.sh check

evidence-regen:
	bash tools/golden.sh regen

evidence-check:
	bash tools/golden.sh check
