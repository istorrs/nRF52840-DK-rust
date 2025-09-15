# SoftDevice S140 Setup Guide

This guide explains how to set up Nordic SoftDevice S140 for BLE functionality with the nRF52840-DK Embassy template.

## What is SoftDevice?

SoftDevice is Nordic Semiconductor's Bluetooth Low Energy protocol stack that provides:
- Certified Bluetooth 5.x implementation
- Concurrent peripheral/central/observer/broadcaster roles
- Thread-safe API for application interaction
- Optimized power consumption
- Real-time guarantees

## SoftDevice S140 v7.3.0 Specifications

- **Target**: nRF52832, nRF52840 (Cortex-M4F)
- **Flash usage**: 160KB (0x27000 bytes)
- **RAM usage**: 128KB (varies with configuration)
- **Concurrent connections**: Up to 20 (configurable)
- **Bluetooth version**: 5.x compliant

## Installation Steps

### 1. Download SoftDevice

1. Go to [Nordic Semiconductor S140 Downloads](https://www.nordicsemi.com/Products/Development-software/S140/Download)
2. Download **s140_nrf52_7.3.0_softdevice.hex**
3. Extract to your project directory

### 2. Flash SoftDevice (One-time Setup)

```bash
# Connect nRF52840-DK via USB
# Erase the flash completely
probe-rs erase --chip nRF52840_xxAA

# Flash SoftDevice (replace with actual path)
probe-rs download --verify --binary-format hex --chip nRF52840_xxAA s140_nrf52_7.3.0_softdevice.hex
```

### 3. Verify Installation

After flashing SoftDevice, you should see:
```
✅ Successfully flashed SoftDevice S140 v7.3.0
```

### 4. Flash Application

Now you can flash the Embassy application:
```bash
make flash
```

## Memory Layout

After SoftDevice installation, memory is partitioned as:

```
Flash Memory (1MB total):
├── 0x00000000 - 0x00026FFF: SoftDevice S140 (160KB)
└── 0x00027000 - 0x000FFFFF: Application space (868KB)

RAM Memory (256KB total):  
├── 0x20000000 - 0x2001FFFF: SoftDevice S140 (128KB)
└── 0x20020000 - 0x2003FFFF: Application space (128KB)
```

## Troubleshooting

### "SoftDevice not present" Error
- **Cause**: SoftDevice not flashed or corrupted
- **Solution**: Re-flash SoftDevice following steps above

### "Invalid memory access" Error  
- **Cause**: Application trying to access SoftDevice memory
- **Solution**: Check `memory.x` file has correct layout

### "Advertisement failed" Error
- **Cause**: SoftDevice configuration mismatch
- **Solution**: Verify SoftDevice version matches code expectations

### Flash Conflicts
- **Cause**: Trying to flash application without SoftDevice
- **Solution**: Always flash SoftDevice first

## Alternative Installation Methods

### Using nRF Connect Programmer
1. Install [nRF Connect for Desktop](https://www.nordicsemi.com/Products/Development-tools/nRF-Connect-for-desktop)
2. Open "Programmer" app  
3. Select nRF52840-DK device
4. Load `s140_nrf52_7.3.0_softdevice.hex`
5. Click "Write"

### Using nrfjprog (Nordic Command Line Tools)
```bash
# Erase chip
nrfjprog --eraseall -f nrf52

# Flash SoftDevice
nrfjprog --program s140_nrf52_7.3.0_softdevice.hex -f nrf52

# Reset
nrfjprog --reset -f nrf52
```

## Development Workflow

Once SoftDevice is installed:

1. **Normal development**: Just flash application with `make flash`
2. **Full reset needed**: Re-flash both SoftDevice + application  
3. **SoftDevice updates**: Erase chip, flash new SoftDevice, then application

## Important Notes

⚠️ **SoftDevice is persistent** - it stays on the chip until explicitly erased

⚠️ **Version compatibility** - Application must match SoftDevice version

⚠️ **Memory constraints** - Application has reduced flash/RAM available

✅ **One-time setup** - SoftDevice flashing only needed once per board

✅ **Development friendly** - Application can be updated without touching SoftDevice

---

For more details, see the [nrf-softdevice documentation](https://github.com/embassy-rs/nrf-softdevice).