# Makefile for building cpclib-vscode extension for different platforms
#
# Usage:
#   make build           - Auto-detect OS and build for current platform
#   make build-linux     - Build for Linux
#   make build-windows   - Build for Windows
#   make build-macosx    - Build for macOS
#   make package-all     - Package extension with binaries for all platforms
#   make clean           - Clean build artifacts

# ============================================================================
# Variables
# ============================================================================

# Project root (one level up from cpclib-vscode)
PROJECT_ROOT := $(shell dirname $(realpath $(lastword $(MAKEFILE_LIST))))
WORKSPACE_ROOT := $(realpath $(PROJECT_ROOT)/..)

# Extension directories
BIN_DIR := $(PROJECT_ROOT)/bin
OUT_DIR := $(PROJECT_ROOT)/out

# Platform-specific variables
LINUX_TARGET := x86_64-unknown-linux-gnu
WINDOWS_TARGET := x86_64-pc-windows-msvc
MACOSX_TARGET := x86_64-apple-darwin

LINUX_BINARY := cpclib-lsp
WINDOWS_BINARY := cpclib-lsp.exe
MACOSX_BINARY := cpclib-lsp

LINUX_BIN_DIR := $(BIN_DIR)/linux
WINDOWS_BIN_DIR := $(BIN_DIR)/windows
MACOSX_BIN_DIR := $(BIN_DIR)/macos

# Cargo build command
CARGO := cargo
CARGO_BUILD := $(CARGO) build --release -p cpclib-lsp

# Node/npm commands
NPM := npm
NPM_CI := $(NPM) ci
NPM_COMPILE := $(NPM) run compile
VSCE := npx @vscode/vsce

# Detect current OS
UNAME_S := $(shell uname -s)
ifeq ($(UNAME_S),Linux)
    DETECTED_OS := linux
endif
ifeq ($(UNAME_S),Darwin)
    DETECTED_OS := macosx
endif
ifeq ($(OS),Windows_NT)
    DETECTED_OS := windows
endif

# ============================================================================
# Functions
# ============================================================================

# Function to build LSP for a specific platform
# Args: $(1) = target triple, $(2) = binary name, $(3) = bin output dir
define build_lsp
	@echo "===> Building cpclib-lsp for target $(1)..."
	cd $(WORKSPACE_ROOT) && $(CARGO_BUILD) --target $(1)
	@echo "===> Creating directory $(3)..."
	mkdir -p $(3)
	@echo "===> Copying binary to $(3)/$(2)..."
	cp $(WORKSPACE_ROOT)/target/$(1)/release/$(2) $(3)/$(2)
	@if [ "$(2)" != "$(WINDOWS_BINARY)" ]; then \
		chmod +x $(3)/$(2); \
	fi
	@echo "===> cpclib-lsp for $(1) built successfully"
endef

# Function to prepare extension (install deps + compile)
define prepare_extension
	@echo "===> Installing npm dependencies..."
	cd $(PROJECT_ROOT) && $(NPM_CI)
	@echo "===> Compiling TypeScript..."
	cd $(PROJECT_ROOT) && $(NPM_COMPILE)
	@echo "===> Extension prepared successfully"
endef

# ============================================================================
# Main targets
# ============================================================================

.PHONY: help
help:
	@echo "CPClib VSCode Extension Build System"
	@echo ""
	@echo "Available targets:"
	@echo "  make build           - Auto-detect OS and build for current platform"
	@echo "  make build-linux     - Build extension with Linux LSP binary"
	@echo "  make build-windows   - Build extension with Windows LSP binary"
	@echo "  make build-macosx    - Build extension with macOS LSP binary"
	@echo "  make package-all     - Build LSP for all platforms and package extension"
	@echo "  make clean           - Remove build artifacts"
	@echo "  make clean-bins      - Remove only platform binaries"
	@echo ""
	@echo "Detected OS: $(DETECTED_OS)"

.PHONY: build
build: build-$(DETECTED_OS)

# ============================================================================
# Platform-specific build targets
# ============================================================================

.PHONY: build-linux
build-linux: clean-bins
	$(call build_lsp,$(LINUX_TARGET),$(LINUX_BINARY),$(LINUX_BIN_DIR))
	$(call prepare_extension)
	@echo ""
	@echo "✓ Linux extension build complete!"
	@echo "  Binary location: $(LINUX_BIN_DIR)/$(LINUX_BINARY)"
	@echo "  To package: cd $(PROJECT_ROOT) && $(VSCE) package"

.PHONY: build-windows
build-windows: clean-bins
	$(call build_lsp,$(WINDOWS_TARGET),$(WINDOWS_BINARY),$(WINDOWS_BIN_DIR))
	$(call prepare_extension)
	@echo ""
	@echo "✓ Windows extension build complete!"
	@echo "  Binary location: $(WINDOWS_BIN_DIR)/$(WINDOWS_BINARY)"
	@echo "  To package: cd $(PROJECT_ROOT) && $(VSCE) package"

.PHONY: build-macosx
build-macosx: clean-bins
	$(call build_lsp,$(MACOSX_TARGET),$(MACOSX_BINARY),$(MACOSX_BIN_DIR))
	$(call prepare_extension)
	@echo ""
	@echo "✓ macOS extension build complete!"
	@echo "  Binary location: $(MACOSX_BIN_DIR)/$(MACOSX_BINARY)"
	@echo "  To package: cd $(PROJECT_ROOT) && $(VSCE) package"

# ============================================================================
# Multi-platform package target
# ============================================================================

.PHONY: package-all
package-all: clean-bins
	@echo "===> Building cpclib-lsp for all platforms..."
	$(call build_lsp,$(LINUX_TARGET),$(LINUX_BINARY),$(LINUX_BIN_DIR))
	$(call build_lsp,$(WINDOWS_TARGET),$(WINDOWS_BINARY),$(WINDOWS_BIN_DIR))
	$(call build_lsp,$(MACOSX_TARGET),$(MACOSX_BINARY),$(MACOSX_BIN_DIR))
	$(call prepare_extension)
	@echo ""
	@echo "===> Packaging extension with all platform binaries..."
	cd $(PROJECT_ROOT) && $(VSCE) package
	@echo ""
	@echo "✓ Multi-platform extension package complete!"
	@echo "  Package: $(PROJECT_ROOT)/*.vsix"

# ============================================================================
# Utility targets
# ============================================================================

.PHONY: install-deps
install-deps:
	@echo "===> Installing npm dependencies..."
	cd $(PROJECT_ROOT) && $(NPM_CI)

.PHONY: compile
compile:
	@echo "===> Compiling TypeScript..."
	cd $(PROJECT_ROOT) && $(NPM_COMPILE)

.PHONY: watch
watch:
	@echo "===> Starting TypeScript watch mode..."
	cd $(PROJECT_ROOT) && $(NPM) run watch

.PHONY: clean-bins
clean-bins:
	@echo "===> Cleaning platform binaries..."
	rm -rf $(BIN_DIR)

.PHONY: clean
clean: clean-bins
	@echo "===> Cleaning build artifacts..."
	rm -rf $(OUT_DIR)
	rm -rf $(PROJECT_ROOT)/node_modules
	rm -f $(PROJECT_ROOT)/*.vsix
	@echo "✓ Clean complete!"

# ============================================================================
# Development targets
# ============================================================================

.PHONY: dev
dev: build
	@echo ""
	@echo "Development build complete. You can now:"
	@echo "  1. Open VSCode"
	@echo "  2. Press F5 to launch Extension Development Host"

.PHONY: package
package:
	@echo "===> Packaging extension..."
	cd $(PROJECT_ROOT) && $(VSCE) package
	@echo "✓ Package created: $(PROJECT_ROOT)/*.vsix"
