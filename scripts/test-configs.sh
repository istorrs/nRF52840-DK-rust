#!/bin/bash

# Test script for nRF52840-DK Embassy Template - All Application Configurations
# This script tests all 4 application configurations to verify they build correctly

set -e  # Exit on any error

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Change to project root directory
cd "$(dirname "$0")/.."

echo -e "${BLUE}=========================================="
echo "nRF52840-DK Embassy Template Config Test"
echo -e "==========================================${NC}"
echo ""
echo "Testing all 4 application configurations..."
echo ""

# Track results
RESULTS=()

# Function to test a configuration
test_config() {
    local config_name="$1"
    local make_target="$2"
    local description="$3"
    local binary_name="$4"

    echo -e "${YELLOW}Configuration: $config_name${NC}"
    echo "Description: $description"
    echo "Make target: $make_target"
    echo "Expected binary: $binary_name"
    echo "------------------------------------------"

    if make "$make_target"; then
        echo -e "${GREEN}✅ Build successful${NC}"

        # Verify binary exists
        if [ -f "target/thumbv7em-none-eabihf/debug/$binary_name" ]; then
            echo -e "${GREEN}✅ Binary found: target/thumbv7em-none-eabihf/debug/$binary_name${NC}"
            RESULTS+=("$config_name: PASS")
        else
            echo -e "${RED}❌ Binary not found: target/thumbv7em-none-eabihf/debug/$binary_name${NC}"
            RESULTS+=("$config_name: FAIL (no binary)")
        fi
    else
        echo -e "${RED}❌ Build failed${NC}"
        RESULTS+=("$config_name: FAIL (build)")
    fi
    echo ""
}

# Test Configuration 1: GPIO-Only App (Default)
# Note: Must build only the main binary to avoid BLE binaries that require ble feature
echo -e "${YELLOW}Configuration: GPIO-Only App${NC}"
echo "Description: Basic GPIO control without BLE (main.rs)"
echo "Make target: cargo build --bin nrf52840-dk-template"
echo "Expected binary: nrf52840-dk-template"
echo "------------------------------------------"

if cargo build --bin nrf52840-dk-template; then
    echo -e "${GREEN}✅ Build successful${NC}"

    # Verify binary exists
    if [ -f "target/thumbv7em-none-eabihf/debug/nrf52840-dk-template" ]; then
        echo -e "${GREEN}✅ Binary found: target/thumbv7em-none-eabihf/debug/nrf52840-dk-template${NC}"
        RESULTS+=("GPIO-Only App: PASS")
    else
        echo -e "${RED}❌ Binary not found: target/thumbv7em-none-eabihf/debug/nrf52840-dk-template${NC}"
        RESULTS+=("GPIO-Only App: FAIL (no binary)")
    fi
else
    echo -e "${RED}❌ Build failed${NC}"
    RESULTS+=("GPIO-Only App: FAIL (build)")
fi
echo ""

# Test Configuration 2: SoftDevice-Compatible GPIO App
test_config \
    "SoftDevice-Compatible GPIO" \
    "build-gpio-sd" \
    "GPIO control with SoftDevice memory layout (gpio_app)" \
    "gpio_app"

# Test Configuration 3: BLE + GPIO Combined App
test_config \
    "BLE + GPIO Combined" \
    "build-ble" \
    "Full BLE functionality with GPIO control (ble_gpio)" \
    "ble_gpio"

# Test Configuration 4: BLE Scanner App
test_config \
    "BLE Scanner" \
    "build-ble-scan" \
    "Dedicated BLE scanning and discovery (ble_scan)" \
    "ble_scan"

# Test build-all target
echo -e "${YELLOW}Testing build-all target...${NC}"
echo "------------------------------------------"
if make clean && make build-all; then
    echo -e "${GREEN}✅ build-all successful${NC}"
    RESULTS+=("build-all target: PASS")
else
    echo -e "${RED}❌ build-all failed${NC}"
    RESULTS+=("build-all target: FAIL")
fi
echo ""

# Summary
echo -e "${BLUE}=========================================="
echo "Test Results Summary"
echo -e "==========================================${NC}"
echo ""

PASS_COUNT=0
FAIL_COUNT=0

for result in "${RESULTS[@]}"; do
    if [[ $result == *"PASS"* ]]; then
        echo -e "${GREEN}✅ $result${NC}"
        ((PASS_COUNT++))
    else
        echo -e "${RED}❌ $result${NC}"
        ((FAIL_COUNT++))
    fi
done

echo ""
echo -e "${BLUE}Total: $((PASS_COUNT + FAIL_COUNT)) tests${NC}"
echo -e "${GREEN}Passed: $PASS_COUNT${NC}"
echo -e "${RED}Failed: $FAIL_COUNT${NC}"

if [ $FAIL_COUNT -eq 0 ]; then
    echo ""
    echo -e "${GREEN}🎉 All configurations build successfully!${NC}"
    echo ""
    echo -e "${BLUE}Next Steps:${NC}"
    echo "1. Flash and test each configuration:"
    echo "   make flash-gpio      # Test GPIO-only"
    echo "   make flash-gpio-sd   # Test SoftDevice-compatible GPIO"
    echo "   make setup-ble       # One-time SoftDevice setup (for BLE apps)"
    echo "   make flash-ble-scan  # Test BLE scanner"
    echo "   make flash-ble       # Test BLE + GPIO combined"
    echo ""
    echo "2. Monitor RTT logs:"
    echo "   make debug-gpio      # Debug GPIO-only"
    echo "   make debug-ble-scan  # Debug BLE scanner"
    echo "   make debug-ble       # Debug BLE + GPIO"
    echo ""
    exit 0
else
    echo ""
    echo -e "${RED}❌ Some configurations failed. Check build errors above.${NC}"
    exit 1
fi