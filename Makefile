SHELL := /bin/sh

.DEFAULT_GOAL := help

SCRIPT_DIR := cilux/vm/scripts
PNPM_VERSION := 10.33.0

define run_pnpm
	@if command -v pnpm >/dev/null 2>&1; then \
		pnpm $(1); \
	elif command -v corepack >/dev/null 2>&1; then \
		corepack pnpm $(1); \
	else \
		npx --yes pnpm@$(PNPM_VERSION) $(1); \
	fi
endef

.PHONY: \
	help \
	setup \
	submodules \
	docs-install \
	preflight \
	fetch-deps \
	fetch-alpine \
	fetch-codex \
	build \
	build-guest \
	build-kernel \
	assemble-initramfs \
	run \
	launch \
	wait-ready \
	test-e2e \
	stop \
	e2e \
	docs-dev \
	docs-build \
	docs-preview \
	deploy-docs

help: ## Show available targets.
	@printf "Targets:\n"
	@awk 'BEGIN {FS = ":.*## "}; /^[a-zA-Z0-9_.-]+:.*## / {printf "  %-18s %s\n", $$1, $$2}' $(MAKEFILE_LIST)

setup: submodules docs-install preflight fetch-deps ## One-shot repo bootstrap: submodules, docs deps, preflight, and guest downloads.

submodules: ## Initialize and update git submodules.
	git submodule update --init --recursive

docs-install: ## Install the root docs dependencies with the pinned pnpm version.
	$(call run_pnpm,install --frozen-lockfile)

preflight: ## Validate host requirements and install the Rust target/components the harness needs.
	./$(SCRIPT_DIR)/preflight.sh

fetch-deps: fetch-alpine fetch-codex ## Download the guest Alpine rootfs and Codex tarball.

fetch-alpine: ## Download the Alpine minirootfs archive used for the guest initramfs.
	./$(SCRIPT_DIR)/fetch-alpine.sh >/dev/null

fetch-codex: ## Download the guest Codex release tarball.
	./$(SCRIPT_DIR)/fetch-codex.sh >/dev/null

build: preflight build-guest build-kernel assemble-initramfs ## Build the guest binaries, kernel, and initramfs.

build-guest: ## Build the guest-side Rust binaries.
	./$(SCRIPT_DIR)/build-guest.sh

build-kernel: ## Build the ARM64 guest kernel in Docker.
	./$(SCRIPT_DIR)/build-kernel.sh

assemble-initramfs: ## Assemble the initramfs from the built guest artifacts.
	./$(SCRIPT_DIR)/assemble-initramfs.sh

run: launch wait-ready ## Launch the VM and wait for the guest app-server to report ready.

launch: ## Launch the guest VM.
	./$(SCRIPT_DIR)/launch.sh

wait-ready: ## Wait for the guest app-server to become ready on localhost:8765.
	./$(SCRIPT_DIR)/wait-ready.sh

test-e2e: ## Run the host-side websocket end-to-end check.
	python3 cilux/tests/app_server_e2e.py

stop: ## Stop the guest VM if it is running.
	./$(SCRIPT_DIR)/stop.sh

e2e: ## Run the full end-to-end harness flow.
	./$(SCRIPT_DIR)/run-e2e.sh

docs-dev: ## Start the docs Vite dev server.
	$(call run_pnpm,run docs:dev)

docs-build: ## Build the docs site into dist/docs-site.
	$(call run_pnpm,run docs:build)

docs-preview: ## Preview the built docs site locally.
	$(call run_pnpm,run docs:preview)

deploy-docs: ## Build and dispatch the GitHub Pages deploy flow.
	$(call run_pnpm,run deploy)
