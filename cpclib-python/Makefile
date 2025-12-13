# Makefile to build the cpclib-python pyo3 extension using maturin
# Usage:
#   make develop      -> build and install into current python env (editable)
#   make build        -> build wheels (release)
#   make wheel        -> alias for build
#   make clean        -> remove build artifacts
#   make rebuild      -> clean + develop
#   make test         -> build (develop) then run pytest in this crate

PYTHON ?= python3
MATURIN ?= maturin
RUSTFLAGS ?=

.PHONY: help develop build wheel clean rebuild test

help:
	@echo "Makefile targets: develop build wheel clean rebuild test"
	@echo "  PYTHON=$(PYTHON) MATURIN=$(MATURIN)"

develop:
	@echo "Running: $(MATURIN) develop -i $(PYTHON) --release"
	$(MATURIN) develop --release

build:
	@echo "Running: $(MATURIN) build -i $(PYTHON) --release"
	$(MATURIN) build --release

wheel: build

rebuild: clean develop

clean:
	@echo "Cleaning build artifacts..."
	-rm -rf target/ build/ dist/ *.egg-info wheelhouse/ || true

test: develop
	@echo "Running pytest for cpclib-python"
	$(PYTHON) -m pytest -q tests
