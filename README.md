# nRF52840-DK Embassy Template

A comprehensive Rust project template for the nRF52840-DK development board featuring Embassy async framework with multiple application configurations and multi-board support. Supports GPIO-only, BLE functionality, and combined GPIO+BLE applications with dynamic board targeting.

## 🚀 Features

- **Multiple App Configurations**: GPIO-only, BLE scanner, GPIO+BLE combined apps
- **Multi-Board Support**: Dynamic detection and targeting of multiple connected nRF52840-DK boards
- **Embassy Async Framework**: Modern async/await embedded programming
- **GPIO Control**: LED patterns, button handling with responsive polling
- **BLE Support**: Complete BLE scanning and GPIO integration
- **SoftDevice Compatible**: Proper Nordic SoftDevice S140 integration
- **Power Efficient**: Automatic low-power mode when idle
- **Easy Debugging**: VS Code integration with RTT logging
- **Flexible Build System**: Individual or combined app building

## 📋 Prerequisites

### Ubuntu/Linux Setup

1. **Install Rust and Toolchain**:
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env
rustup target add thumbv7em-none-eabihf
```

2. **Install system dependencies**:
```bash
sudo apt update
sudo apt install -y build-essential libudev-dev pkg-config
```

3. **Install probe-rs** (replaces OpenOCD):
```bash
cargo install probe-rs-tools
```

4. **Install additional tools**:
```bash
cargo install flip-link
```

5. **Setup udev rules** for nRF52840-DK:
```bash
sudo tee /etc/udev/rules.d/69-probe-rs.rules > /dev/null <<'EOF'
# nRF52840-DK
SUBSYSTEM=="usb", ATTR{idVendor}=="1366", ATTR{idProduct}=="1015", MODE="0664", GROUP="plugdev"
EOF
sudo udevadm control --reload-rules
sudo udevadm trigger
```

## 🔧 Quick Start

1. **Clone or download this template**
2. **Connect your nRF52840-DK** via USB
3. **Setup the environment**:
```bash
make setup
```

4. **Choose your application and build/flash**:

**GPIO-only app (default)**:
```bash
make flash-gpio          # or just: make flash
```

**BLE + GPIO combined app**:
```bash
make setup-ble          # One-time SoftDevice setup
make flash-ble
```

**BLE scanner app**:
```bash
make setup-ble          # One-time SoftDevice setup
make flash-ble-scan
```

**CLI interface app**:
```bash
make setup-ble          # One-time SoftDevice setup
make flash-cli
```

**SoftDevice-compatible GPIO app**:
```bash
make flash-gpio-sd
```

5. **Start debugging with RTT logs**:
```bash
make debug-gpio         # or: make debug-ble, make debug-ble-scan, make debug-cli
```

## 🎯 Multiple Board Support

This template supports multiple nRF52840-DK boards connected simultaneously. Board selection is automatic and dynamic.

### Board Detection and Selection

**List connected boards**:
```bash
make list-boards        # Show available boards
probe-rs list          # Detailed probe information
```

**Target specific board** (default is board 0):
```bash
# Use default board (0)
make flash-gpio
make debug-ble

# Target specific board
make flash-gpio BOARD=1
make debug-ble-scan BOARD=0
make setup-ble BOARD=1
```

### Examples with Multiple Boards

**Scenario: Two boards for BLE communication testing**
```bash
# Setup both boards with SoftDevice
make setup-ble BOARD=0
make setup-ble BOARD=1

# Flash different apps to each board
make flash-ble BOARD=0        # Board 0: BLE + GPIO combined
make flash-ble-scan BOARD=1   # Board 1: BLE scanner

# Debug both simultaneously (separate terminals)
make debug-ble BOARD=0        # Terminal 1: Monitor BLE + GPIO
make debug-ble-scan BOARD=1   # Terminal 2: Monitor BLE scanning
```

**Error Handling**: The system provides clear error messages if boards are not found:
```
Board 2 not found. Available boards: 0-1. Run 'probe-rs list' for details.
No boards detected. Please connect an nRF52840-DK and run 'probe-rs list'
```

## 🏗️ Project Structure

```
nRF52840-DK-rust/
├── src/
│   ├── main.rs              # GPIO-only app (default)
│   ├── gpio_tasks.rs        # Shared GPIO task implementations
│   ├── cli/                 # CLI interface modules
│   │   ├── mod.rs           # CLI module definitions
│   │   ├── terminal.rs      # Terminal I/O handling
│   │   ├── parser.rs        # Command parsing and autocompletion
│   │   └── commands.rs      # Command execution handlers
│   └── bin/
│       ├── gpio_app.rs      # SoftDevice-compatible GPIO app
│       ├── ble_gpio.rs      # BLE + GPIO combined app
│       ├── ble_scan.rs      # BLE scanner app
│       └── cli_app.rs       # CLI interface app
├── .cargo/config.toml       # Cargo configuration for nRF52840
├── Cargo.toml              # Dependencies (Embassy, nrf-softdevice)
├── memory-*.x              # Memory layouts for different configurations
├── build.rs                # Build script for memory layout selection
├── Embed.toml              # probe-rs configuration
├── .vscode/                # VS Code debug configuration
│   ├── launch.json         # Debug profiles
│   └── settings.json       # Rust analyzer settings
├── scripts/                # Utility scripts
└── Makefile                # Multi-app build commands
```

## 📱 Application Configurations

### 1. GPIO-Only App (`main.rs`)
- **Purpose**: Basic GPIO control without BLE
- **Features**: LED patterns, button handling, RTT logging
- **Memory**: Uses full memory layout (no SoftDevice)
- **Build**: `make build-gpio` or `make build`

### 2. SoftDevice-Compatible GPIO App (`src/bin/gpio_app.rs`)
- **Purpose**: GPIO control that preserves SoftDevice memory space
- **Features**: Same GPIO functionality, SoftDevice-compatible interrupt priorities
- **Memory**: Uses SoftDevice-preserving memory layout
- **Build**: `make build-gpio-sd`

### 3. BLE + GPIO Combined App (`src/bin/ble_gpio.rs`)
- **Purpose**: Full BLE functionality with GPIO control
- **Features**: BLE scanning, GPIO patterns, combined operation
- **Memory**: Uses SoftDevice memory layout
- **Requires**: SoftDevice S140 v7.3.0 flashed first
- **Build**: `make build-ble`

### 4. BLE Scanner App (`src/bin/ble_scan.rs`)
- **Purpose**: Dedicated BLE scanning and device discovery
- **Features**: BLE advertisement scanning, device information logging
- **Memory**: Uses SoftDevice memory layout
- **Requires**: SoftDevice S140 v7.3.0 flashed first
- **Build**: `make build-ble-scan`

### 5. CLI Interface App (`src/bin/cli_app.rs`)
- **Purpose**: Interactive command-line interface via UART
- **Features**: Command autocompletion, BLE control, GPIO control, system status
- **Interface**: UART1 (pins P1.14/P1.15) at 115200 baud
- **Commands**: help, version, status, uptime, clear, reset, echo, led_on/off, button, temp, bt_on, bt_off, bt_scan
- **Memory**: Uses SoftDevice memory layout (required for BLE commands)
- **Requires**: SoftDevice S140 v7.3.0 flashed first
- **Build**: `make build-cli`

## 🎮 Hardware Mapping (nRF52840-DK)

### LEDs (Active Low)
- **LED1** (P0.13): Heartbeat indicator
- **LED2** (P0.14): Button press indicator
- **LED3** (P0.15): Pattern LED 1
- **LED4** (P0.16): Pattern LED 2

### Buttons (Active Low, Pull-up)
- **BUTTON1** (P0.11): Controls LED2
- **BUTTON2** (P0.12): Available for custom use
- **BUTTON3** (P0.24): Available for custom use
- **BUTTON4** (P0.25): Available for custom use

## 📱 BLE Functionality

### BLE Scanner App (`ble_scan.rs`)
- **Purpose**: Scans for nearby BLE devices and logs advertisement data
- **Output**: Device addresses, connection status, advertisement data via RTT
- **Usage**: Ideal for BLE environment discovery and debugging

### BLE + GPIO Combined App (`ble_gpio.rs`)
- **Purpose**: Combines BLE scanning with GPIO control
- **Features**:
  - Simultaneous BLE device scanning
  - LED heartbeat and button handling
  - GPIO pattern display
- **Advertisement**: Device advertises as **"nRF52840-DK-GPIO"**
- **Usage**: Full-featured application demonstrating BLE/GPIO coexistence

### 📋 SoftDevice Requirements

**IMPORTANT**: This template requires Nordic SoftDevice S140 v7.3.0 to be flashed before the application:

1. **Download SoftDevice**: Get `s140_nrf52_7.3.0_softdevice.hex` from [Nordic Semiconductor](https://www.nordicsemi.com/Products/Development-software/S140/Download)
2. **One-time setup**: Flash SoftDevice once per board with `make setup-ble BOARD=N`
3. **Memory layout**: Application starts at `0x27000` (after SoftDevice)
4. **Preservation**: BLE apps use `probe-rs download` to preserve SoftDevice during flashing

### Phone Connection Example

Use any BLE scanner app (nRF Connect, BLE Scanner) to:

1. Scan for "nRF52840-DK-GPIO" (BLE + GPIO app)
2. View discovered devices and their advertisement data
3. Monitor RTT logs for detailed BLE activity

## 🛠️ Development Commands

**All commands support board selection with `BOARD=N` parameter (default: BOARD=0)**

### App-Specific Commands
```bash
# GPIO-only applications
make build-gpio          # Build GPIO-only app
make flash-gpio          # Flash GPIO-only app
make debug-gpio          # Debug GPIO-only app

make build-gpio-sd       # Build SoftDevice-compatible GPIO app
make flash-gpio-sd       # Flash SoftDevice-compatible GPIO app
make debug-gpio-sd       # Debug SoftDevice-compatible GPIO app

# BLE applications (require SoftDevice setup)
make build-ble           # Build BLE + GPIO app
make flash-ble           # Flash BLE + GPIO app
make debug-ble           # Debug BLE + GPIO app

make build-ble-scan      # Build BLE scanner app
make flash-ble-scan      # Flash BLE scanner app
make debug-ble-scan      # Debug BLE scanner app

make build-cli           # Build CLI interface app
make flash-cli           # Flash CLI interface app
make debug-cli           # Debug CLI interface app

# Utility commands
make build-all           # Build all applications
make setup-ble           # Setup SoftDevice S140 (one-time)
make list-boards         # List connected nRF52840-DK boards
```

### Legacy Commands (Default to GPIO-only)
```bash
make build               # Build GPIO-only app (default)
make release             # Build release version (GPIO-only)
make flash               # Flash GPIO-only app (default)
make flash-release       # Flash release version (GPIO-only)
make debug               # Debug GPIO-only app (default)

# Maintenance commands
make format              # Format code
make check               # Run code checks (clippy + format)
make test-configs        # Test all application configurations
make release-test        # Complete release test sequence
make clean               # Clean build artifacts
make help                # Show all available commands
```

## 🔍 Debugging

### RTT Logging
Real-Time Transfer (RTT) provides fast, non-intrusive logging:

```bash
make debug  # Start RTT session
```

### VS Code Integration

The template includes multiple debug configurations to support different VS Code extensions and debugging workflows:

**Recommended Extensions** (auto-suggested when opening project):
- `rust-lang.rust-analyzer` - Rust language server
- `marus25.cortex-debug` - ARM Cortex debugging
- `ms-vscode.cpptools` - C/C++ debugging support  
- `probe-rs.probe-rs-debugger` - probe-rs debugging

**Available Debug Configurations**:

1. **Debug nRF52840-DK (Cortex-Debug + OpenOCD)** - Uses OpenOCD with Cortex-Debug extension
2. **Attach to nRF52840-DK (Cortex-Debug + OpenOCD)** - Attach to running target
3. **Debug nRF52840-DK (probe-rs)** - Uses probe-rs debugger extension  
4. **Debug nRF52840-DK (Cortex-Debug + J-Link)** - For J-Link debug probes
5. **Debug nRF52840-DK (Native GDB)** - Uses GDB with OpenOCD backend

**Quick Start**:
1. Install recommended extensions (VS Code will prompt automatically)
2. Open project in VS Code
3. Press **F5** or go to **Run and Debug** panel
4. Select your preferred debug configuration
5. Breakpoints, variable inspection, and RTT logs work seamlessly

**Troubleshooting VS Code Debug**:
- If "debug type not supported" error appears, install the corresponding extension
- For probe-rs configuration, ensure `probe-rs-debugger` extension is installed
- For OpenOCD configurations, ensure `cortex-debug` extension is installed

### Manual probe-rs Commands
```bash
# Flash and run different apps
probe-rs run --chip nRF52840_xxAA target/thumbv7em-none-eabihf/debug/nrf52840-dk-template  # GPIO-only
probe-rs run --chip nRF52840_xxAA target/thumbv7em-none-eabihf/debug/gpio_app              # GPIO + SoftDevice

# BLE apps (preserve SoftDevice)
probe-rs download --chip nRF52840_xxAA target/thumbv7em-none-eabihf/debug/ble_gpio         # BLE + GPIO (download only)
probe-rs download --chip nRF52840_xxAA target/thumbv7em-none-eabihf/debug/ble_scan         # BLE scanner (download only)

# Attach for debugging (after download)
probe-rs attach --chip nRF52840_xxAA target/thumbv7em-none-eabihf/debug/ble_gpio    # For BLE + GPIO app
probe-rs attach --chip nRF52840_xxAA target/thumbv7em-none-eabihf/debug/ble_scan    # For BLE scanner app
```

## ⚡ Power Management

Embassy automatically handles power management:
- **Sleep**: CPU sleeps when no tasks are ready
- **Wake**: Interrupts wake the system efficiently  
- **Low Power**: Designed for battery-powered applications

## 🔧 Customization

### Adding New GPIO
Edit `src/gpio_tasks.rs`:
```rust
#[task]
pub async fn my_custom_task(mut pin: Output<'static>) {
    // Your async GPIO logic here
}
```

### Extending BLE Services
Edit `src/ble_task.rs`:
```rust
#[nrf_softdevice::gatt_service(uuid = "your-custom-uuid")]
pub struct MyCustomService {
    #[characteristic(uuid = "char-uuid", read, write, notify)]
    pub my_data: Vec<u8, 32>,
}
```

### Adding Sensors
Embassy-nrf supports many peripherals:
- **I2C**: `embassy_nrf::twim`
- **SPI**: `embassy_nrf::spim`  
- **ADC**: `embassy_nrf::saadc`
- **PWM**: `embassy_nrf::pwm`

## 🚨 Troubleshooting

### Common Issues

**"probe-rs not found"**
```bash
cargo install probe-rs --features cli
```

**"Permission denied" USB**
```bash
sudo usermod -a -G plugdev $USER
# Logout and login again
```

**Build errors with memory**
- Check `memory.x` matches your SoftDevice version
- Ensure SoftDevice S140 is flashed first

**BLE not advertising**
- Verify SoftDevice S140 is present
- Check RTT logs for BLE initialization errors

**"Core is locked" or erase errors**
- For BLE apps, use `make flash-ble` or `make debug-ble` (preserves SoftDevice)
- Don't use `probe-rs run` directly for BLE apps (erases SoftDevice)
- Re-flash SoftDevice with `make setup-ble` if accidentally erased

### Getting Help
- Embassy documentation: https://embassy.dev  
- nRF52840-DK user guide: Nordic Semiconductor docs
- probe-rs documentation: https://probe.rs

## 📄 License

This project is licensed under either of

 * Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
 * MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

---

🦀 **Happy Embedded Rust Development!** 🚀