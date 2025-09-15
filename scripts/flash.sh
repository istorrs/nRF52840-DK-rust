#!/bin/bash

# Flash script for nRF52840-DK with Embassy template
# This script builds and flashes the application to the board

set -e

echo "🔥 Building and flashing nRF52840-DK Embassy application..."

# Check if probe-rs is installed
if ! command -v probe-rs &> /dev/null; then
    echo "❌ probe-rs not found. Please install it first:"
    echo "   cargo install probe-rs --features cli"
    exit 1
fi

# Build the project
echo "🔧 Building project..."
cargo build --release

# Flash the application
echo "📱 Flashing to nRF52840-DK..."
probe-rs run --chip nRF52840_xxAA target/thumbv7em-none-eabihf/release/nrf52840-dk-template

echo "✅ Flash complete! The application should now be running."
echo "📡 To see RTT logs, run: probe-rs attach --chip nRF52840_xxAA"