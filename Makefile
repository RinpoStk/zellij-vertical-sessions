RUSTUP ?= rustup
RUSTUP_CARGO := $(shell $(RUSTUP) which cargo 2>/dev/null)
RUSTUP_RUSTC := $(shell $(RUSTUP) which rustc 2>/dev/null)
RUSTUP_CLIPPY := $(shell $(RUSTUP) which clippy-driver 2>/dev/null)

CARGO ?= $(if $(RUSTUP_CARGO),$(RUSTUP_CARGO),cargo)
RUSTC ?= $(if $(RUSTUP_RUSTC),$(RUSTUP_RUSTC),rustc)
CLIPPY_DRIVER ?= $(RUSTUP_CLIPPY)

TARGET ?= wasm32-wasip1
PROFILE ?= release
PLUGIN_NAME := zellij-vertical-sessions
WASM := target/$(TARGET)/$(PROFILE)/$(PLUGIN_NAME).wasm
INSTALL_DIR ?= $(HOME)/.config/zellij/plugins

.PHONY: all build release debug check clippy install clean fmt target-installed clippy-installed

all: release

build: release

release: PROFILE := release
release: target-installed
	RUSTC="$(RUSTC)" "$(CARGO)" build --release --target $(TARGET)

debug: PROFILE := debug
debug: target-installed
	RUSTC="$(RUSTC)" "$(CARGO)" build --target $(TARGET)

check: target-installed
	RUSTC="$(RUSTC)" "$(CARGO)" check --target $(TARGET)

clippy: target-installed clippy-installed
	CLIPPY_DRIVER="$(CLIPPY_DRIVER)" RUSTC="$(RUSTC)" "$(CARGO)" clippy --target $(TARGET) -- -D warnings

fmt:
	"$(CARGO)" fmt --all

install: release
	mkdir -p "$(INSTALL_DIR)"
	cp "$(WASM)" "$(INSTALL_DIR)/$(PLUGIN_NAME).wasm"

clean:
	"$(CARGO)" clean

target-installed:
	@libdir="$$(RUSTC="$(RUSTC)" "$(RUSTC)" --print target-libdir --target "$(TARGET)" 2>/dev/null)"; \
	test -n "$$libdir" && ls "$$libdir"/libcore-*.rlib >/dev/null 2>&1 || { \
		echo "Rust target '$(TARGET)' is not installed."; \
		echo "Install it with: $(RUSTUP) target add $(TARGET)"; \
		exit 1; \
	}

clippy-installed:
	@test -n "$(CLIPPY_DRIVER)" || { \
		echo "Rust clippy component is not installed for the active rustup toolchain."; \
		echo "Install it with: $(RUSTUP) component add clippy"; \
		exit 1; \
	}
