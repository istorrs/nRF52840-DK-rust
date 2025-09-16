# CLAUDE.md - Claude Code Development Context

This file provides Claude Code with essential information about this nRF52840-DK Embassy template project for efficient development assistance.

## Project Overview

**nRF52840-DK Embassy Template** - A comprehensive Rust embedded project template featuring multiple application configurations for the Nordic nRF52840-DK development board.

### Key Technologies
- **Embassy Async Framework**: Modern async/await embedded programming
- **nRF52840**: Nordic ARM Cortex-M4 microcontroller
- **SoftDevice S140**: Nordic BLE stack (v7.3.0)
- **probe-rs**: Modern debugging and flashing tool
- **RTT**: Real-Time Transfer for logging

## Application Configurations

The project supports 4 distinct application configurations:

### 1. GPIO-Only App (`src/main.rs`)
- **Purpose**: Basic GPIO control without BLE functionality
- **Features**: LED patterns, button handling, RTT logging
- **Memory**: Uses full memory layout (no SoftDevice required)
- **Build**: `make build-gpio` or `make build`
- **Flash**: `make flash-gpio` or `make flash`

### 2. SoftDevice-Compatible GPIO App (`src/bin/gpio_app.rs`)
- **Purpose**: GPIO control that preserves SoftDevice memory space
- **Features**: Same GPIO functionality with SoftDevice-compatible interrupt priorities
- **Memory**: Uses SoftDevice-preserving memory layout
- **Build**: `make build-gpio-sd`
- **Flash**: `make flash-gpio-sd`

### 3. BLE + GPIO Combined App (`src/bin/ble_gpio.rs`)
- **Purpose**: Full BLE functionality with GPIO control
- **Features**: BLE scanning, GPIO patterns, combined operation
- **Memory**: Uses SoftDevice memory layout
- **Requirements**: SoftDevice S140 v7.3.0 must be flashed first
- **Build**: `make build-ble`
- **Flash**: `make flash-ble`

### 4. BLE Scanner App (`src/bin/ble_scan.rs`)
- **Purpose**: Dedicated BLE scanning and device discovery
- **Features**: BLE advertisement scanning, device logging
- **Memory**: Uses SoftDevice memory layout
- **Requirements**: SoftDevice S140 v7.3.0 must be flashed first
- **Build**: `make build-ble-scan`
- **Flash**: `make flash-ble-scan`

## Build System Architecture

### Memory Layouts (`build.rs`)
The project uses different memory layouts selected by feature flags:
- **`memory-no-softdevice.x`**: Full memory for GPIO-only app
- **`memory-gpio-with-softdevice.x`**: SoftDevice-preserving layout for GPIO apps
- **`memory-softdevice.x`**: SoftDevice layout for BLE apps

### Feature Flags
- **`ble`**: Enables BLE functionality, selects SoftDevice memory layout
- **`gpio`**: Enables GPIO functionality, selects SoftDevice-preserving layout
- **No features**: Default GPIO-only app with full memory

### Cargo Binaries
- **Default binary**: `src/main.rs` (GPIO-only)
- **Named binaries**:
  - `gpio_app` → `src/bin/gpio_app.rs`
  - `ble_gpio` → `src/bin/ble_gpio.rs`
  - `ble_scan` → `src/bin/ble_scan.rs`

## Common Development Tasks

### Building and Flashing
```bash
# Build specific app
cargo build --bin ble_gpio --features ble

# Flash using Makefile (recommended)
make flash-ble

# Direct probe-rs usage
probe-rs run --chip nRF52840_xxAA target/thumbv7em-none-eabihf/debug/ble_gpio
```

### Debugging
```bash
# Start RTT debugging session
make debug-ble

# VS Code debugging
# Use F5 with multiple debug configurations available
```

### SoftDevice Setup (One-time)
```bash
make setup-ble  # Downloads and flashes SoftDevice S140 v7.3.0
```

## Hardware Configuration (nRF52840-DK)

### GPIO Mapping
- **LEDs** (Active Low): P0.13 (LED1), P0.14 (LED2), P0.15 (LED3), P0.16 (LED4)
- **Buttons** (Pull-up): P0.11 (BTN1), P0.12 (BTN2), P0.24 (BTN3), P0.25 (BTN4)

### LED Functionality
- **LED1**: Heartbeat indicator (100ms on, 900ms off)
- **LED2**: Button press indicator (BTN1 control)
- **LED3/LED4**: Alternating pattern (300ms each)

## Interrupt Priority Configuration

**Critical for SoftDevice compatibility**:
- SoftDevice reserves interrupt priorities 0 and 1
- Embassy and application interrupts must use priority 2 or lower
- Current configuration: `Priority::P2` for GPIOTE and timer interrupts

## Known Issues and Fixes

### UART Hang Issue (RESOLVED)
- **Problem**: UART write operations caused hangs in BLE applications
- **Solution**: Removed all UART code, using only RTT for logging
- **Files affected**: All main application files now use RTT exclusively

### SoftDevice Initialization Order (RESOLVED)
- **Problem**: `SdmIncorrectInterruptConfiguration` error
- **Solution**: Initialize Embassy before SoftDevice with compatible interrupt priorities
- **Critical**: Embassy init → SoftDevice enable → peripheral initialization

### Memory Layout Selection (AUTOMATED)
- **Feature-based selection**: `build.rs` automatically selects correct memory layout
- **No manual intervention**: Developers just use feature flags with cargo/make

## Testing and Validation

### Test Each Configuration
```bash
make build-all          # Verify all apps compile
make flash-gpio         # Test GPIO-only
make flash-gpio-sd      # Test SoftDevice-compatible GPIO
make flash-ble-scan     # Test BLE scanner
make flash-ble          # Test BLE + GPIO combined
```

### Verification Steps
1. **GPIO-only**: LEDs blink, button controls LED2, RTT logs appear
2. **BLE scanner**: Discovers nearby BLE devices, logs to RTT
3. **BLE + GPIO**: Both BLE scanning and GPIO patterns work simultaneously

## Development Workflow

### Adding New Features
1. **Choose target app**: Determine which configuration needs the feature
2. **Edit appropriate file**: `main.rs`, `gpio_app.rs`, `ble_gpio.rs`, or `ble_scan.rs`
3. **Update Makefile**: If needed, add new build targets
4. **Test all configurations**: Ensure changes don't break other apps

### Memory Optimization
- **Check SoftDevice warning**: RTT logs show if too much RAM allocated to SoftDevice
- **Adjust memory layout**: Modify appropriate `memory-*.x` file if needed
- **Verify all apps**: Ensure memory changes work across configurations

## VS Code Integration

### Debug Configurations
Multiple debug profiles available in `.vscode/launch.json`:
- Cortex-Debug + OpenOCD
- probe-rs debugger
- J-Link support
- GDB with OpenOCD

### Extensions
Required/recommended extensions auto-suggested:
- `rust-lang.rust-analyzer`
- `marus25.cortex-debug`
- `probe-rs.probe-rs-debugger`

## Troubleshooting Guide

### Common Build Issues
1. **Memory layout errors**: Check feature flags match intended memory layout
2. **SoftDevice not found**: Run `make setup-ble` for BLE applications
3. **probe-rs not found**: Install with `cargo install probe-rs-tools`

### Runtime Issues
1. **App hangs during init**: Check interrupt priority configuration
2. **BLE not working**: Verify SoftDevice is flashed and memory layout is correct
3. **GPIO not responding**: Ensure correct pin assignments in hardware mapping

## Make Command Reference

### App-Specific Commands
- `make build-gpio`, `make flash-gpio`, `make debug-gpio`
- `make build-gpio-sd`, `make flash-gpio-sd`, `make debug-gpio-sd`
- `make build-ble`, `make flash-ble`, `make debug-ble`
- `make build-ble-scan`, `make flash-ble-scan`, `make debug-ble-scan`

### Utility Commands
- `make build-all` - Build all applications
- `make setup-ble` - One-time SoftDevice setup
- `make help` - Show all available commands

This template provides a solid foundation for nRF52840 development with clear separation of concerns and flexible configuration options.