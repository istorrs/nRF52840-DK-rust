# Makefile for nRF52840-DK Embassy Template
# Provides convenient commands for building, flashing, and debugging multiple app configurations

# Colors for output
GREEN=\033[0;32m
BLUE=\033[0;34m
NC=\033[0m

# Board selection (default to 0 if not specified)
BOARD ?= 0
# Check if any boards are detected
BOARDS_DETECTED = $(shell probe-rs list 2>/dev/null | grep -c "^\[.*\]:" || echo 0)
# Dynamically get probe selector for the specified board
PROBE_SELECTOR = $(shell probe-rs list 2>/dev/null | grep "^\[$(BOARD)\]:" | sed 's/.*-- \([^:]*:[^:]*:[^[:space:]]*\).*/\1/')
PROBE_ARG = $(if $(PROBE_SELECTOR),--probe $(PROBE_SELECTOR),$(if $(filter 0,$(BOARDS_DETECTED)),$(error No boards detected. Please connect an nRF52840-DK and run 'probe-rs list'),$(error Board $(BOARD) not found. Available boards: 0-$(shell echo $$(($(BOARDS_DETECTED)-1))). Run 'probe-rs list' for details)))

.PHONY: all build flash debug clean setup setup-probe-rs setup-ble help format check test-configs release-test list-boards
.PHONY: build-gpio build-gpio-sd build-ble build-ble-scan build-cli
.PHONY: flash-gpio flash-gpio-sd flash-ble flash-ble-scan flash-cli
.PHONY: debug-gpio debug-gpio-sd debug-ble debug-ble-scan debug-cli

# Default target - GPIO-only app
all: build-gpio

# === Build Targets ===

# Build GPIO-only app (main.rs - no SoftDevice)
build-gpio:
	@echo "🔧 Building GPIO-only app..."
	cargo build --bin nrf52840-dk-template

# Build GPIO app with SoftDevice compatibility
build-gpio-sd:
	@echo "🔧 Building SoftDevice-compatible GPIO app..."
	cargo build --bin gpio_app --features gpio

# Build BLE + GPIO combined app
build-ble:
	@echo "🔧 Building BLE + GPIO app..."
	cargo build --bin ble_gpio --no-default-features --features ble

# Build BLE scan app
build-ble-scan:
	@echo "🔧 Building BLE scanner app..."
	cargo build --bin ble_scan --no-default-features --features ble

# Build CLI app
build-cli:
	@echo "🔧 Building CLI app..."
	cargo build --bin cli_app --no-default-features --features cli

# Build all apps
build-all:
	@echo "🔧 Building all apps..."
	@make build-gpio
	@make build-gpio-sd
	@make build-ble
	@make build-ble-scan
	@make build-cli

# === Flash Targets ===

# Flash GPIO-only app
flash-gpio: build-gpio
	@echo "📱 Flashing GPIO-only app to board $(BOARD)..."
	probe-rs run $(PROBE_ARG) --chip nRF52840_xxAA target/thumbv7em-none-eabihf/debug/nrf52840-dk-template

# Flash SoftDevice-compatible GPIO app
flash-gpio-sd: build-gpio-sd
	@echo "📱 Flashing SoftDevice-compatible GPIO app to board $(BOARD)..."
	probe-rs run $(PROBE_ARG) --chip nRF52840_xxAA target/thumbv7em-none-eabihf/debug/gpio_app

# Flash BLE + GPIO app (preserves SoftDevice)
flash-ble: build-ble
	@echo "📱 Flashing BLE + GPIO app to board $(BOARD) (preserving SoftDevice)..."
	probe-rs download $(PROBE_ARG) --chip nRF52840_xxAA target/thumbv7em-none-eabihf/debug/ble_gpio

# Flash BLE scan app (preserves SoftDevice)
flash-ble-scan: build-ble-scan
	@echo "📱 Flashing BLE scanner app to board $(BOARD) (preserving SoftDevice)..."
	probe-rs download $(PROBE_ARG) --chip nRF52840_xxAA target/thumbv7em-none-eabihf/debug/ble_scan

# Flash CLI app (preserves SoftDevice)
flash-cli: build-cli
	@echo "📱 Flashing CLI app to board $(BOARD) (preserving SoftDevice)..."
	probe-rs download $(PROBE_ARG) --chip nRF52840_xxAA target/thumbv7em-none-eabihf/debug/cli_app

# === Debug Targets ===

# Debug GPIO-only app
debug-gpio: build-gpio
	@echo "🐛 Starting debug session (GPIO-only) on board $(BOARD)..."
	probe-rs run $(PROBE_ARG) --chip nRF52840_xxAA target/thumbv7em-none-eabihf/debug/nrf52840-dk-template

# Debug SoftDevice-compatible GPIO app
debug-gpio-sd: build-gpio-sd
	@echo "🐛 Starting debug session (GPIO + SoftDevice) on board $(BOARD)..."
	probe-rs run $(PROBE_ARG) --chip nRF52840_xxAA target/thumbv7em-none-eabihf/debug/gpio_app

# Debug BLE + GPIO app (preserves SoftDevice)
debug-ble: flash-ble
	@echo "🐛 Starting debug session (BLE + GPIO) on board $(BOARD)..."
	probe-rs attach $(PROBE_ARG) --chip nRF52840_xxAA target/thumbv7em-none-eabihf/debug/ble_gpio

# Debug BLE scan app (preserves SoftDevice)
debug-ble-scan: flash-ble-scan
	@echo "🐛 Starting debug session (BLE scanner) on board $(BOARD)..."
	probe-rs attach $(PROBE_ARG) --chip nRF52840_xxAA target/thumbv7em-none-eabihf/debug/ble_scan

# Debug CLI app (preserves SoftDevice)
debug-cli: flash-cli
	@echo "🐛 Starting debug session (CLI app) on board $(BOARD)..."
	probe-rs attach $(PROBE_ARG) --chip nRF52840_xxAA target/thumbv7em-none-eabihf/debug/cli_app

# === Legacy Targets (for backward compatibility) ===

# Build default (GPIO-only)
build: build-gpio

# Flash default (GPIO-only)
flash: flash-gpio

# Debug default (GPIO-only)
debug: debug-gpio

# Build release version (GPIO-only)
release:
	@echo "🔧 Building release version (GPIO-only)..."
	cargo build --release --bin nrf52840-dk-template

# Flash release version (GPIO-only)
flash-release: release
	@echo "📱 Flashing release version (GPIO-only) to board $(BOARD)..."
	probe-rs run $(PROBE_ARG) --chip nRF52840_xxAA target/thumbv7em-none-eabihf/release/nrf52840-dk-template

# Clean build artifacts
clean:
	@echo "🧹 Cleaning build artifacts..."
	cargo clean

# Setup development environment
setup:
	@echo "⚙️  Setting up development environment..."
	@echo "Installing Rust embedded toolchain..."
	rustup target add thumbv7em-none-eabihf
	@echo "Installing flip-link..."
	cargo install flip-link
	@echo ""
	@echo "📋 probe-rs installation info:"
	@echo "   For development/compilation: Not required - project builds without it"
	@echo "   For flashing to hardware: System dependencies needed:"
	@echo "     sudo apt update && sudo apt install -y libudev-dev pkg-config"
	@echo "     cargo install probe-rs-tools"
	@echo ""
	@echo "💡 Alternative: Use existing tools like OpenOCD, Black Magic Probe, or nRF Connect"
	@echo ""
	@echo "✅ Setup complete! Project ready for development."

# Install probe-rs (run after installing system dependencies)
setup-probe-rs:
	@echo "🔧 Installing probe-rs..."
	@echo "Note: This requires system dependencies (libudev-dev, pkg-config)"
	@command -v pkg-config >/dev/null 2>&1 && pkg-config --exists libudev || { \
		echo "❌ System dependencies missing. Run:"; \
		echo "   sudo apt update && sudo apt install -y libudev-dev pkg-config"; \
		exit 1; \
	}
	cargo install probe-rs-tools
	@echo "✅ probe-rs installation complete!"

# Setup BLE functionality (flash SoftDevice S140)
setup-ble:
	@echo "🔧 Setting up BLE functionality on board $(BOARD)..."
	@echo "This will download and flash SoftDevice S140 v7.3.0"
	PROBE_ARG="$(PROBE_ARG)" ./scripts/flash-softdevice.sh

# Format code
format:
	@echo "🎨 Formatting code..."
	cargo fmt

# List connected boards
list-boards:
	@echo "🔍 Connected nRF52840-DK boards:"
	@if command -v probe-rs >/dev/null 2>&1; then \
		if probe-rs list 2>/dev/null | grep -q "^\[.*\]:"; then \
			probe-rs list | grep "^\[.*\]:"; \
		else \
			echo "❌ No boards detected. Please connect an nRF52840-DK via USB."; \
		fi \
	else \
		echo "❌ probe-rs not installed. Run 'make setup-probe-rs' first."; \
	fi

# Check code (clippy + format check)
check:
	@echo "🔍 Checking code..."
	cargo fmt -- --check
	@echo "Checking GPIO-only configuration..."
	cargo clippy --bin nrf52840-dk-template -- -D warnings
	@echo "Checking SoftDevice-compatible GPIO configuration..."
	cargo clippy --bin gpio_app --features gpio -- -D warnings
	@echo "Checking BLE configurations..."
	cargo clippy --bin ble_gpio --bin ble_scan --no-default-features --features ble -- -D warnings

# Test all configurations
test-configs:
	@echo "🧪 Testing all application configurations..."
	./scripts/test-configs.sh

# Complete release test sequence
release-test:
	@echo "🚀 Running complete release test sequence..."
	@echo ""
	@echo "Step 1/5: Code quality and standards check..."
	@make check
	@echo ""
	@echo "Step 2/5: Format code..."
	@make format
	@echo ""
	@echo "Step 3/5: Test all configurations..."
	@make test-configs
	@echo ""
	@echo "Step 4/5: Clean build artifacts..."
	@make clean
	@echo ""
	@echo "Step 5/5: Full rebuild of all configurations..."
	@make build-all
	@echo ""
	@echo -e "${GREEN}🎉 Release test completed successfully!${NC}"
	@echo ""
	@echo -e "${BLUE}Optional hardware verification steps:${NC}"
	@echo "  make setup-ble       # One-time SoftDevice setup"
	@echo "  make debug-gpio      # Test GPIO-only with RTT"
	@echo "  make debug-ble-scan  # Test BLE scanner with RTT"
	@echo "  make debug-ble       # Test BLE+GPIO with RTT"
	@echo ""
	@echo -e "${BLUE}Release build verification:${NC}"
	@echo "  make release         # Test optimized release build"

# Erase chip completely (removes SoftDevice)
erase-chip:
	@echo "🔥 WARNING: This will completely erase the chip!"
	@echo "This will remove:"
	@echo "  - SoftDevice S140"
	@echo "  - All applications"
	@echo "  - All protection settings"
	@echo ""
	@read -p "Continue? (y/N): " confirm && [ "$$confirm" = "y" ] || exit 1
	@echo "🔥 Erasing chip..."
	probe-rs erase --chip nRF52840_xxAA --allow-erase-all
	@echo "✅ Chip erased. You can now flash GPIO-only apps with 'make flash-gpio'"

# Recover locked chip (for APPROTECT issues)
recover-chip:
	@echo "🔓 Attempting to recover locked chip..."
	@echo "This will try multiple recovery methods:"
	@echo "  1. probe-rs recover"
	@echo "  2. probe-rs erase with force"
	@echo "  3. nrfjprog recover (if available)"
	@echo ""
	@read -p "Continue? (y/N): " confirm && [ "$$confirm" = "y" ] || exit 1
	@echo "🔓 Trying probe-rs recover..."
	-probe-rs recover --chip nRF52840_xxAA
	@echo "🔓 Trying probe-rs erase with force..."
	-probe-rs erase --chip nRF52840_xxAA --allow-erase-all
	@echo "🔓 Trying nrfjprog recover (if available)..."
	-nrfjprog --family nrf52 --recover
	@echo "🔓 Trying nrfjprog erase..."
	-nrfjprog --family nrf52 --eraseall
	@echo "✅ Recovery attempt complete. Try 'make flash-gpio' to test."

# Show help
help:
	@echo "nRF52840-DK Embassy Template - Available Commands:"
	@echo ""
	@echo "=== Board Selection ==="
	@echo "  BOARD=N              - Target specific board (default: 0)"
	@echo "  Board indices are auto-detected from 'probe-rs list'"
	@echo "  Examples:"
	@echo "    make flash-gpio BOARD=0   # Flash to board [0]"
	@echo "    make debug-ble BOARD=1    # Debug BLE on board [1]"
	@echo "    make setup-ble BOARD=1    # Setup SoftDevice on board [1]"
	@echo "    make list-boards          # List connected boards"
	@echo "    probe-rs list             # Full probe details"
	@echo ""
	@echo "=== App-Specific Commands ==="
	@echo "  make build-gpio      - Build GPIO-only app (main.rs)"
	@echo "  make build-gpio-sd   - Build SoftDevice-compatible GPIO app"
	@echo "  make build-ble       - Build BLE + GPIO combined app"
	@echo "  make build-ble-scan  - Build BLE scanner app"
	@echo "  make build-cli       - Build CLI app with USB CDC interface"
	@echo "  make build-all       - Build all apps"
	@echo ""
	@echo "  make flash-gpio      - Flash GPIO-only app"
	@echo "  make flash-gpio-sd   - Flash SoftDevice-compatible GPIO app"
	@echo "  make flash-ble       - Flash BLE + GPIO combined app"
	@echo "  make flash-ble-scan  - Flash BLE scanner app"
	@echo "  make flash-cli       - Flash CLI app"
	@echo ""
	@echo "  make debug-gpio      - Debug GPIO-only app"
	@echo "  make debug-gpio-sd   - Debug SoftDevice-compatible GPIO app"
	@echo "  make debug-ble       - Debug BLE + GPIO combined app"
	@echo "  make debug-ble-scan  - Debug BLE scanner app"
	@echo "  make debug-cli       - Debug CLI app"
	@echo ""
	@echo "=== Legacy Commands (default to GPIO-only) ==="
	@echo "  make build           - Build GPIO-only app (default)"
	@echo "  make release         - Build release version (GPIO-only)"
	@echo "  make flash           - Flash GPIO-only app (default)"
	@echo "  make flash-release   - Flash release version (GPIO-only)"
	@echo "  make debug           - Debug GPIO-only app (default)"
	@echo ""
	@echo "=== Utility Commands ==="
	@echo "  make clean           - Clean build artifacts"
	@echo "  make setup           - Install required tools (basic)"
	@echo "  make setup-probe-rs  - Install probe-rs (after system deps)"
	@echo "  make setup-ble       - Setup BLE (flash SoftDevice S140)"
	@echo "  make format          - Format source code"
	@echo "  make check           - Check code formatting and lints"
	@echo "  make test-configs    - Test all application configurations"
	@echo "  make release-test    - Complete release test sequence"
	@echo "  make list-boards     - List connected nRF52840-DK boards"
	@echo "  make erase-chip      - Completely erase chip (removes SoftDevice)"
	@echo "  make recover-chip    - Recover locked chip (APPROTECT issues)"
	@echo "  make help            - Show this help message"