.PHONY: test rust fmt clippy evidence-regen evidence-check

# prefer cargo on PATH, fall back to the rustup default location
CARGO := $(shell command -v cargo 2>/dev/null || echo $$HOME/.cargo/bin/cargo)

rust:
	$(CARGO) test --workspace

fmt:
	$(CARGO) fmt --all -- --check

clippy:
	$(CARGO) clippy --workspace --all-targets -- -D warnings

# Full local test suite: Rust + Python probe compile + golden corpus.
# The golden part self-skips when CIVTOT_HOME is unset (e.g. CI).
test: rust
	python3 -m py_compile tools/totctl.py tools/growth_series.py tools/finance_check.py tools/research_check.py tools/connections_check.py tools/trade_routes_check.py tools/production_check.py tools/wonders_check.py tools/happiness_check.py tools/maintenance_check.py tools/huts_check.py
	bash tools/golden.sh check

evidence-regen:
	bash tools/golden.sh regen

evidence-check:
	bash tools/golden.sh check
