# Makefile for nRF52840-DK Embassy Template
# Provides convenient commands for building, flashing, and debugging multiple app configurations

.PHONY: all build flash debug clean setup setup-probe-rs setup-ble help format check test-configs
.PHONY: build-gpio build-gpio-sd build-ble build-ble-scan
.PHONY: flash-gpio flash-gpio-sd flash-ble flash-ble-scan
.PHONY: debug-gpio debug-gpio-sd debug-ble debug-ble-scan

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

# Build all apps
build-all:
	@echo "🔧 Building all apps..."
	@make build-gpio
	@make build-gpio-sd
	@make build-ble
	@make build-ble-scan

# === Flash Targets ===

# Flash GPIO-only app
flash-gpio: build-gpio
	@echo "📱 Flashing GPIO-only app..."
	probe-rs run --chip nRF52840_xxAA target/thumbv7em-none-eabihf/debug/nrf52840-dk-template

# Flash SoftDevice-compatible GPIO app
flash-gpio-sd: build-gpio-sd
	@echo "📱 Flashing SoftDevice-compatible GPIO app..."
	probe-rs run --chip nRF52840_xxAA target/thumbv7em-none-eabihf/debug/gpio_app

# Flash BLE + GPIO app
flash-ble: build-ble
	@echo "📱 Flashing BLE + GPIO app..."
	probe-rs run --chip nRF52840_xxAA target/thumbv7em-none-eabihf/debug/ble_gpio

# Flash BLE scan app
flash-ble-scan: build-ble-scan
	@echo "📱 Flashing BLE scanner app..."
	probe-rs run --chip nRF52840_xxAA target/thumbv7em-none-eabihf/debug/ble_scan

# === Debug Targets ===

# Debug GPIO-only app
debug-gpio: build-gpio
	@echo "🐛 Starting debug session (GPIO-only)..."
	probe-rs run --chip nRF52840_xxAA target/thumbv7em-none-eabihf/debug/nrf52840-dk-template

# Debug SoftDevice-compatible GPIO app
debug-gpio-sd: build-gpio-sd
	@echo "🐛 Starting debug session (GPIO + SoftDevice)..."
	probe-rs run --chip nRF52840_xxAA target/thumbv7em-none-eabihf/debug/gpio_app

# Debug BLE + GPIO app
debug-ble: build-ble
	@echo "🐛 Starting debug session (BLE + GPIO)..."
	probe-rs run --chip nRF52840_xxAA target/thumbv7em-none-eabihf/debug/ble_gpio

# Debug BLE scan app
debug-ble-scan: build-ble-scan
	@echo "🐛 Starting debug session (BLE scanner)..."
	probe-rs run --chip nRF52840_xxAA target/thumbv7em-none-eabihf/debug/ble_scan

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
	@echo "📱 Flashing release version (GPIO-only)..."
	probe-rs run --chip nRF52840_xxAA target/thumbv7em-none-eabihf/release/nrf52840-dk-template

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
	@echo "🔧 Setting up BLE functionality..."
	@echo "This will download and flash SoftDevice S140 v7.3.0"
	./scripts/flash-softdevice.sh

# Format code
format:
	@echo "🎨 Formatting code..."
	cargo fmt

# Check code (clippy + format check)
check:
	@echo "🔍 Checking code..."
	cargo fmt -- --check
	cargo clippy -- -D warnings

# Test all configurations
test-configs:
	@echo "🧪 Testing all application configurations..."
	./scripts/test-configs.sh

# Show help
help:
	@echo "nRF52840-DK Embassy Template - Available Commands:"
	@echo ""
	@echo "=== App-Specific Commands ==="
	@echo "  make build-gpio      - Build GPIO-only app (main.rs)"
	@echo "  make build-gpio-sd   - Build SoftDevice-compatible GPIO app"
	@echo "  make build-ble       - Build BLE + GPIO combined app"
	@echo "  make build-ble-scan  - Build BLE scanner app"
	@echo "  make build-all       - Build all apps"
	@echo ""
	@echo "  make flash-gpio      - Flash GPIO-only app"
	@echo "  make flash-gpio-sd   - Flash SoftDevice-compatible GPIO app"
	@echo "  make flash-ble       - Flash BLE + GPIO combined app"
	@echo "  make flash-ble-scan  - Flash BLE scanner app"
	@echo ""
	@echo "  make debug-gpio      - Debug GPIO-only app"
	@echo "  make debug-gpio-sd   - Debug SoftDevice-compatible GPIO app"
	@echo "  make debug-ble       - Debug BLE + GPIO combined app"
	@echo "  make debug-ble-scan  - Debug BLE scanner app"
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
	@echo "  make help            - Show this help message"