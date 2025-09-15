#!/bin/bash

# Flash Nordic SoftDevice S140 v7.3.0 for BLE functionality
# This script downloads and flashes the SoftDevice required for BLE operations

set -e

SOFTDEVICE_VERSION="7.3.0"
SOFTDEVICE_FILE="s140_nrf52_${SOFTDEVICE_VERSION}_softdevice.hex"

# Nordic's direct download URLs change frequently, so we provide manual instructions
NORDIC_DOWNLOAD_PAGE="https://www.nordicsemi.com/Products/Development-software/S140/Download"

echo "🔧 nRF52840-DK SoftDevice S140 v${SOFTDEVICE_VERSION} Flash Script"
echo ""

# Check if probe-rs is available
if ! command -v probe-rs &> /dev/null; then
    echo "❌ probe-rs not found. Please install it first:"
    echo "   cargo install probe-rs-tools"
    exit 1
fi

# Check if nRF52840-DK is connected
if ! probe-rs list | grep -q "J-Link"; then
    echo "❌ nRF52840-DK not detected. Please:"
    echo "   1. Connect your nRF52840-DK via USB"
    echo "   2. Ensure USB permissions are set (run ./scripts/fix-usb-permissions.sh)"
    exit 1
fi

echo "✅ nRF52840-DK detected"

# Check if SoftDevice file exists
if [ ! -f "$SOFTDEVICE_FILE" ]; then
    echo "📥 SoftDevice S140 v${SOFTDEVICE_VERSION} not found locally"
    echo ""
    echo "📋 Please download SoftDevice S140 v${SOFTDEVICE_VERSION} manually:"
    echo "   1. Visit: ${NORDIC_DOWNLOAD_PAGE}"
    echo "   2. Download: s140_nrf52_${SOFTDEVICE_VERSION}_softdevice.hex"
    echo "   3. Save it in this directory as: ${SOFTDEVICE_FILE}"
    echo ""
    echo "💡 Alternatively, if you have the file elsewhere, copy it here:"
    echo "   cp /path/to/your/${SOFTDEVICE_FILE} ."
    echo ""
    exit 1
else
    echo "✅ SoftDevice file found: $SOFTDEVICE_FILE"
fi

# Verify file exists and is not empty
if [ ! -s "$SOFTDEVICE_FILE" ]; then
    echo "❌ SoftDevice file is empty or corrupt. Please re-download manually."
    exit 1
fi

echo ""
echo "🚨 WARNING: This will erase the chip and flash SoftDevice S140"
echo "📋 About to:"
echo "   1. Erase entire nRF52840 flash"
echo "   2. Flash SoftDevice S140 v${SOFTDEVICE_VERSION} to chip"
echo "   3. This is required for BLE functionality"
echo ""
read -p "Continue? (y/N): " -n 1 -r
echo
if [[ ! $REPLY =~ ^[Yy]$ ]]; then
    echo "Cancelled by user"
    exit 0
fi

echo ""
echo "🔥 Erasing chip..."
probe-rs erase --chip nRF52840_xxAA --allow-erase-all

echo "📱 Flashing SoftDevice S140 v${SOFTDEVICE_VERSION}..."
probe-rs download --verify --binary-format hex --chip nRF52840_xxAA "$SOFTDEVICE_FILE"

echo ""
echo "✅ SoftDevice S140 v${SOFTDEVICE_VERSION} flashed successfully!"
echo ""
echo "📋 Next steps:"
echo "   1. Update memory.x to use SoftDevice memory layout"
echo "   2. Enable BLE code in main.rs"
echo "   3. Build and flash your application: make flash"
echo ""
echo "💡 The application will now start at address 0x27000 (after SoftDevice)"
echo ""