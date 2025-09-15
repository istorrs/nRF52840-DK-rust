# Makefile for nRF52840-DK Embassy Template
# Provides convenient commands for building, flashing, and debugging

.PHONY: all build flash debug clean setup setup-probe-rs help format check

# Default target
all: build

# Build the project (debug)
build:
	@echo "🔧 Building project..."
	cargo build

# Build release version
release:
	@echo "🔧 Building release version..."
	cargo build --release

# Flash debug version to board
flash: build
	@echo "📱 Flashing debug version..."
	probe-rs run --chip nRF52840_xxAA target/thumbv7em-none-eabihf/debug/nrf52840-dk-template

# Flash release version to board
flash-release: release
	@echo "📱 Flashing release version..."
	probe-rs run --chip nRF52840_xxAA target/thumbv7em-none-eabihf/release/nrf52840-dk-template

# Start debug session with RTT logging
debug: build
	@echo "🐛 Starting debug session..."
	probe-rs attach --chip nRF52840_xxAA

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
	@echo "📋 Manual probe-rs installation required:"
	@echo "   System dependencies needed for probe-rs:"
	@echo "   sudo apt update && sudo apt install -y libudev-dev pkg-config"
	@echo "   cargo install probe-rs-tools"
	@echo ""
	@echo "✅ Basic setup complete! Please install probe-rs manually."

# Install probe-rs (run after installing system dependencies)
setup-probe-rs:
	@echo "🔧 Installing probe-rs..."
	cargo install probe-rs-tools
	@echo "✅ probe-rs installation complete!"

# Format code
format:
	@echo "🎨 Formatting code..."
	cargo fmt

# Check code (clippy + format check)
check:
	@echo "🔍 Checking code..."
	cargo fmt -- --check
	cargo clippy -- -D warnings

# Show help
help:
	@echo "nRF52840-DK Embassy Template - Available Commands:"
	@echo ""
	@echo "  make build         - Build debug version"
	@echo "  make release       - Build release version"
	@echo "  make flash         - Build and flash debug version"
	@echo "  make flash-release - Build and flash release version"
	@echo "  make debug         - Start debug session with RTT"
	@echo "  make clean         - Clean build artifacts"
	@echo "  make setup         - Install required tools (basic)"
	@echo "  make setup-probe-rs - Install probe-rs (after system deps)"
	@echo "  make format        - Format source code"
	@echo "  make check         - Check code formatting and lints"
	@echo "  make help          - Show this help message"