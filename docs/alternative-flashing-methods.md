# Alternative Flashing Methods for nRF52840-DK

If you can't install probe-rs due to system dependency issues, here are alternative methods to flash and debug your nRF52840-DK.

## Option 1: nRF Connect for Desktop (Recommended)

Nordic's official GUI tool - works without command line dependencies.

### Installation
1. Download from [Nordic Semiconductor](https://www.nordicsemi.com/Products/Development-tools/nRF-Connect-for-desktop)
2. Install the AppImage or package for your system
3. Launch and install the "Programmer" app

### Usage
1. **Connect nRF52840-DK** via USB
2. **Open Programmer app**
3. **Select device** (should show nRF52840-DK)
4. **Flash SoftDevice**:
   - Click "Add file" → select `s140_nrf52_7.3.0_softdevice.hex`
   - Click "Write"
5. **Flash application**:
   - Build with `cargo build --release`
   - Convert ELF to HEX: `arm-none-eabi-objcopy -O ihex target/thumbv7em-none-eabihf/release/nrf52840-dk-template firmware.hex`
   - Add `firmware.hex` to programmer
   - Click "Write"

## Option 2: Nordic Command Line Tools

Professional command-line tools from Nordic.

### Installation
1. Download [nRF Command Line Tools](https://www.nordicsemi.com/Products/Development-tools/nrf-command-line-tools)
2. Extract and add to PATH

### Usage
```bash
# Build project
cargo build --release

# Convert to HEX format
arm-none-eabi-objcopy -O ihex \
  target/thumbv7em-none-eabihf/release/nrf52840-dk-template \
  firmware.hex

# Flash SoftDevice (one time)
nrfjprog --eraseall -f nrf52
nrfjprog --program s140_nrf52_7.3.0_softdevice.hex -f nrf52

# Flash application
nrfjprog --program firmware.hex -f nrf52 --sectorerase
nrfjprog --reset -f nrf52
```

## Option 3: OpenOCD + GDB

Traditional open-source debugging tools.

### Installation
```bash
sudo apt install openocd gdb-multiarch
```

### Usage
```bash
# Build project
cargo build

# In terminal 1: Start OpenOCD
openocd -f interface/cmsis-dap.cfg -f target/nrf52.cfg

# In terminal 2: GDB session
gdb-multiarch target/thumbv7em-none-eabihf/debug/nrf52840-dk-template
(gdb) target remote :3333
(gdb) monitor reset halt
(gdb) load
(gdb) continue
```

## Option 4: Black Magic Probe

Hardware debugger probe (if available).

### Usage
```bash
# Build project
cargo build

# Flash with Black Magic Probe
gdb-multiarch target/thumbv7em-none-eabihf/debug/nrf52840-dk-template
(gdb) target extended-remote /dev/ttyACM0
(gdb) monitor swdp_scan
(gdb) attach 1
(gdb) load
(gdb) run
```

## Option 5: VS Code with Cortex-Debug

GUI debugging in VS Code.

### Setup
1. Install VS Code extensions:
   - "Cortex-Debug"
   - "rust-analyzer"

2. Create `.vscode/launch.json`:
```json
{
    "version": "0.2.0",
    "configurations": [
        {
            "name": "Debug nRF52840",
            "type": "cortex-debug",
            "request": "launch",
            "program": "${workspaceFolder}/target/thumbv7em-none-eabihf/debug/nrf52840-dk-template",
            "chip": "nRF52840_xxAA",
            "servertype": "openocd",
            "configFiles": [
                "interface/cmsis-dap.cfg",
                "target/nrf52.cfg"
            ]
        }
    ]
}
```

3. Press F5 to start debugging

## Option 6: Web-based Solutions

### Nordic Device Programming app
- Visit [DevZone Online Programmer](https://devzone.nordicsemi.com/)
- Upload HEX files via web interface
- Flash via connected programmer

## RTT Logging Alternatives

Since probe-rs provides RTT logging, here are alternatives:

### 1. SEGGER RTT Viewer
```bash
# Download SEGGER J-Link Software Pack
# Use RTT Viewer for log output
```

### 2. OpenOCD RTT
```bash
# In OpenOCD session
(gdb) monitor rtt setup 0x20000000 0x10000 "SEGGER RTT"
(gdb) monitor rtt start
(gdb) monitor rtt server start 9090 0
# Connect to localhost:9090 for RTT output
```

### 3. Serial/UART Logging
Modify code to use UART instead of RTT:
```rust
// Replace defmt-rtt with UART logging
use embassy_nrf::uarte;
```

## Development Workflow

Even without probe-rs, you can:

1. **Develop and compile** locally with `cargo build`
2. **Run tests** with `cargo test` 
3. **Use flash tools** when ready to test on hardware
4. **Debug with GDB** for step-through debugging

## Summary

| Method | Pros | Cons |
|--------|------|------|
| nRF Connect | Official, GUI, no deps | Nordic-specific |
| nrfjprog | Fast, scriptable | Nordic-specific |
| OpenOCD | Open source, flexible | Setup complexity |
| Black Magic | Hardware probe, fast | Requires special hardware |
| VS Code | GUI debugging | Extension dependencies |

**Recommendation**: Start with nRF Connect for Desktop for the easiest experience, then move to command-line tools as you get more comfortable.