# pnix-hy — local management gate (same commands CI runs; proposal 0011/0012).
# Run from the repo root:  make -C pnix-hy verify   (or: cd pnix-hy && make verify)
# Set PNIX_HY_PYTHON to a Python that has Hy 1.3.0 for the projection/proof tiers.

PY ?= python3
PYTHONPATH := $(CURDIR)$(if $(PYTHONPATH),:$(PYTHONPATH))
export PYTHONPATH

.PHONY: ci verify check gate capabilities capabilities-check deployment

ci: ## full LOCAL CI (mirrors .github/workflows/pnix-hy-gate.yml; auto-finds a Hy Python)
	bash scripts/ci-local.sh

verify: check gate ## run the full local gate (toolkit self-checks + ship-gate)

capabilities-check: ## fail if committed docs/CAPABILITIES.md is stale (drift gate)
	$(PY) -m pnix_hy.cli --capabilities > /tmp/pnix_hy.CAPABILITIES.md
	mv /tmp/pnix_hy.CAPABILITIES.md docs/CAPABILITIES.md
	git diff --exit-code docs/CAPABILITIES.md

check: ## 57 toolkit self-checks (includes docs<->code drift gate)
	$(PY) -m pnix_hy.cli --check

gate: ## sacred lanes (runtime self-test / rust corpus / 4-lane mirror / closure) + toolkit
	$(PY) -m pnix_hy.cli --gate

capabilities: ## regenerate the code-derived capability index (docs/CAPABILITIES.md)
	$(PY) -m pnix_hy.cli --capabilities > docs/CAPABILITIES.md.tmp
	mv docs/CAPABILITIES.md.tmp docs/CAPABILITIES.md
	@echo "regenerated docs/CAPABILITIES.md"

deployment: ## report which capability tiers work in this install
	$(PY) -m pnix_hy.cli --deployment
