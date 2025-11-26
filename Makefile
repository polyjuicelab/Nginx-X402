.PHONY: help publish prepare-publish clean-nginx-source

# Default nginx version for publishing
NGX_VERSION ?= 1.22.0
NGINX_SOURCE_DIR := /tmp/nginx-$(NGX_VERSION)

help:
	@echo "Available targets:"
	@echo "  make publish          - Publish to crates.io (requires NGX_VERSION or uses default 1.22.0)"
	@echo "  make prepare-publish   - Prepare nginx source for publishing"
	@echo "  make clean-nginx-source - Clean downloaded nginx source"
	@echo ""
	@echo "Environment variables:"
	@echo "  NGX_VERSION           - Nginx version to use (default: 1.22.0)"
	@echo ""
	@echo "Examples:"
	@echo "  make publish"
	@echo "  NGX_VERSION=1.29.0 make publish"
	@echo "  make publish ARGS=\"--allow-dirty\""

prepare-publish:
	@echo "Preparing nginx source for publishing..."
	@echo "Version: $(NGX_VERSION)"
	@echo "Source dir: $(NGINX_SOURCE_DIR)"
	@if [ ! -d "$(NGINX_SOURCE_DIR)/objs" ]; then \
		echo "Downloading nginx-$(NGX_VERSION)..."; \
		cd /tmp; \
		if [ ! -f "nginx-$(NGX_VERSION).tar.gz" ]; then \
			if command -v wget >/dev/null 2>&1; then \
				wget -q "https://nginx.org/download/nginx-$(NGX_VERSION).tar.gz" || exit 1; \
			elif command -v curl >/dev/null 2>&1; then \
				curl -sSfL -o "nginx-$(NGX_VERSION).tar.gz" "https://nginx.org/download/nginx-$(NGX_VERSION).tar.gz" || exit 1; \
			else \
				echo "ERROR: wget or curl not found"; \
				exit 1; \
			fi; \
		fi; \
		if [ ! -d "nginx-$(NGX_VERSION)" ]; then \
			tar -xzf "nginx-$(NGX_VERSION).tar.gz" || exit 1; \
			rm -f "nginx-$(NGX_VERSION).tar.gz"; \
		fi; \
		cd "nginx-$(NGX_VERSION)"; \
		echo "Configuring nginx source..."; \
		./configure --without-http_rewrite_module --with-cc-opt="-fPIC" >/dev/null 2>&1 || { \
			echo "Failed to configure nginx source"; \
			exit 1; \
		}; \
		echo "Configured nginx source"; \
	else \
		echo "Using existing nginx source at $(NGINX_SOURCE_DIR)"; \
	fi

publish: prepare-publish
	@echo ""
	@echo "Environment variables:"
	@echo "  NGINX_SOURCE_DIR=$(NGINX_SOURCE_DIR)"
	@echo "  NGX_VERSION=$(NGX_VERSION)"
	@echo ""
	@echo "Running cargo publish..."
	@NGINX_SOURCE_DIR="$(NGINX_SOURCE_DIR)" NGX_VERSION="$(NGX_VERSION)" cargo publish $(ARGS)

clean-nginx-source:
	@echo "Cleaning nginx source at $(NGINX_SOURCE_DIR)..."
	@rm -rf "$(NGINX_SOURCE_DIR)"
	@rm -f "/tmp/nginx-$(NGX_VERSION).tar.gz"
	@echo "Cleaned nginx source"

