# BLE Integration with Embassy

## Current Status

This template currently provides **GPIO functionality only** with Embassy async framework. BLE integration with Nordic SoftDevice is available but requires additional configuration to resolve interrupt conflicts.

## The Challenge

Embassy and nrf-softdevice both require specific interrupt configurations that can conflict:

- **Embassy** uses RTC1 for its time driver and requires certain interrupt priorities
- **SoftDevice S140** manages its own interrupt configuration and requires specific priorities
- The error `SdmIncorrectInterruptConfiguration` occurs when these conflict

## Working Solutions

### Option 1: GPIO-Only Mode (Current)
✅ **Fully functional** - This template as configured
- Embassy async framework with full GPIO support
- Responsive button handling and LED patterns
- VS Code debugging with RTT logs
- Full Embassy features without BLE

### Option 2: BLE-Only Mode (Alternative)
Use nrf-softdevice without Embassy's async features:
- Nordic SoftDevice S140 for BLE
- Traditional interrupt-driven GPIO
- Raw nRF52840 HAL for peripherals

### Option 3: Embassy + BLE (Future Work)
Requires careful interrupt priority configuration:
- Use Embassy with SoftDevice-compatible settings
- Configure interrupt priorities to avoid conflicts
- May require specific Embassy/nrf-softdevice versions

## BLE Implementation Files

The template includes complete BLE implementation files for future integration:

- `src/ble_task.rs` - Nordic UART Service + Custom Sensor Service
- `scripts/flash-softdevice.sh` - SoftDevice S140 flashing script
- `docs/alternative-flashing-methods.md` - Additional BLE setup info

## Quick BLE Setup (When Ready)

1. **Flash SoftDevice S140**:
   ```bash
   make setup-ble
   ```

2. **Update memory layout**:
   ```rust
   // In memory.x
   FLASH : ORIGIN = 0x00027000, LENGTH = 0x000d9000  // After SoftDevice
   RAM : ORIGIN = 0x20020000, LENGTH = 0x00020000    // After SoftDevice
   ```

3. **Enable BLE code**:
   ```rust
   // In main.rs - restore BLE imports and tasks
   mod ble_task;
   use ble_task::*;
   // ... SoftDevice configuration
   ```

## Embassy + SoftDevice Resources

- [Embassy nRF Documentation](https://docs.embassy.dev/embassy-nrf/)
- [nrf-softdevice Examples](https://github.com/embassy-rs/nrf-softdevice/tree/main/examples)
- [SoftDevice S140 Documentation](https://infocenter.nordicsemi.com/topic/struct_s140/struct/s140_nrf52_api.html)

## Recommended Approach

For most users, the **GPIO-only mode** provides excellent async embedded development experience with Embassy. BLE can be added later when the interrupt configuration is resolved.

For immediate BLE needs, consider using Nordic's official examples or nrf-softdevice without Embassy's time drivers.