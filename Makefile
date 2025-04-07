VENDOR_DIR := vendor
TS_VERSION := 0.20.0
TS_REPO := https://github.com/tree-sitter/tree-sitter.git
LANG_REPOS := \
	https://github.com/tree-sitter/tree-sitter-rust.git \
	https://github.com/tree-sitter/tree-sitter-python.git \
	https://github.com/tree-sitter/tree-sitter-javascript.git \
	https://github.com/tree-sitter/tree-sitter-typescript.git \
	https://github.com/tree-sitter/tree-sitter-java.git \
	https://github.com/tree-sitter/tree-sitter-go.git \
	https://github.com/tree-sitter/tree-sitter-cpp.git \
	https://github.com/tree-sitter/tree-sitter-c.git \
	https://github.com/tree-sitter/tree-sitter-ruby.git \
	https://github.com/tree-sitter/tree-sitter-php.git

# Main targets
.PHONY: all clean build install update test check-dirs

all: setup build

check-dirs:
	@echo "Checking repository structures..."
	@find $(VENDOR_DIR) -name "parser.c" | sort

# Create vendor directory and clone dependencies
setup: $(VENDOR_DIR) tree-sitter languages

$(VENDOR_DIR):
	mkdir -p $(VENDOR_DIR)

tree-sitter: $(VENDOR_DIR)
	@echo "Cloning tree-sitter core library..."
	@if [ ! -d "$(VENDOR_DIR)/tree-sitter" ]; then \
		git clone $(TS_REPO) $(VENDOR_DIR)/tree-sitter; \
		cd $(VENDOR_DIR)/tree-sitter && git checkout v$(TS_VERSION); \
	else \
		echo "tree-sitter already exists, skipping."; \
	fi

languages: $(VENDOR_DIR)
	@echo "Cloning language parsers..."
	@for repo in $(LANG_REPOS); do \
		repo_name=$$(basename "$${repo}" .git); \
		if [ ! -d "$(VENDOR_DIR)/$${repo_name}" ]; then \
			echo "Cloning $${repo_name}..."; \
			git clone "$${repo}" "$(VENDOR_DIR)/$${repo_name}"; \
		else \
			echo "$${repo_name} already exists, skipping."; \
		fi; \
	done

# Build the project
build:
	@echo "Building Relik Indexor..."
	cargo build --release
	cp target/release/relik_codegraph .

# Clean up
clean:
	@echo "Cleaning build artifacts..."
	cargo clean
	@echo "Done cleaning build artifacts."

# Deep clean (removes vendor directory too)
deep-clean: clean
	@echo "Removing vendor directory..."
	rm -rf $(VENDOR_DIR)
	@echo "Done cleaning everything."

# Update dependencies
update: $(VENDOR_DIR)
	@echo "Updating tree-sitter core..."
	@if [ -d "$(VENDOR_DIR)/tree-sitter" ]; then \
		cd $(VENDOR_DIR)/tree-sitter && git fetch && git checkout v$(TS_VERSION); \
	fi
	@echo "Updating language parsers..."
	@for repo in $(LANG_REPOS); do \
		repo_name=$$(basename "$${repo}" .git); \
		if [ -d "$(VENDOR_DIR)/$${repo_name}" ]; then \
			echo "Updating $${repo_name}..."; \
			cd "$(VENDOR_DIR)/$${repo_name}" && git fetch && git pull; \
		fi; \
	done
