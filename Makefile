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
	python3 -m unittest discover -s tools -p 'test_worldgen_compare.py'
	python3 -m unittest discover -s tools -p 'test_preview_client.py'
	python3 -m py_compile tools/preview_client.py
	python3 -m py_compile tools/worldgen_compare.py tools/worldgen_capture_manifest.py tools/oracle/dialog_guard.py tools/ai_research_choice_check.py tools/ai_research_value_check.py tools/ai_research_adapter_check.py tools/ai_production_compare.py tools/ai_campaign_audit.py tools/capture_check.py
	python3 -m py_compile tools/check_client_linux.py tools/check_client_selection.py tools/package_clients.py tools/citizens_check.py
	python3 -m py_compile tools/city_production_check.py tools/happiness_stages_check.py tools/check_city_panels.py
	python3 -m py_compile tools/totctl.py tools/growth_series.py tools/finance_check.py tools/research_check.py tools/connections_check.py tools/trade_routes_check.py tools/production_check.py tools/wonders_check.py tools/happiness_check.py tools/maintenance_check.py tools/huts_check.py tools/calendar_check.py tools/tech_groups_check.py tools/allocation_check.py tools/hut_fixture.py tools/barbarians_check.py tools/power_check.py tools/statistics_check.py tools/leonardo_check.py tools/diplomatic_wonders_check.py tools/apollo_check.py tools/growth_check.py tools/workers_check.py tools/availability_check.py tools/yields_check.py tools/city_totals_check.py tools/starvation_check.py tools/city_phase_check.py tools/city_destruction_check.py tools/mood_transition_check.py tools/support_disband_check.py tools/anarchy_check.py tools/city_route_totals_check.py tools/library_check.py tools/veteran_check.py tools/wonder_yield_check.py tools/movement_allowance_check.py tools/landmass_flags_check.py tools/combat_defense_check.py tools/combat_land_check.py tools/oracle/city_series.py tools/oracle/barbarian_series.py
	bash tools/golden.sh check

evidence-regen:
	bash tools/golden.sh regen

evidence-check:
	bash tools/golden.sh check
