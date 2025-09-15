# nRF52840-DK Embassy Template

A best-in-class Rust project template for the nRF52840-DK development board featuring Embassy async framework, GPIO control, and Bluetooth Low Energy (BLE) connectivity.

## 🚀 Features

- **Embassy Async Framework**: Modern async/await embedded programming
- **GPIO Control**: LED patterns, button handling with debouncing
- **BLE Connectivity**: GATT server for phone data collection
- **Power Efficient**: Automatic low-power mode when idle  
- **Easy Debugging**: VS Code integration with RTT logging
- **One-Command Flashing**: Simple build and deploy workflow

## 📋 Prerequisites

### Ubuntu/Linux Setup

1. **Install Rust and Toolchain**:
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env
rustup target add thumbv7em-none-eabihf
```

2. **Install probe-rs** (replaces OpenOCD):
```bash
cargo install probe-rs --features cli
```

3. **Install additional tools**:
```bash
cargo install flip-link
sudo apt update
sudo apt install build-essential
```

4. **Setup udev rules** for nRF52840-DK:
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

4. **Build and flash**:
```bash
make flash
```

5. **Start debugging with RTT logs**:
```bash
make debug
```

## 🏗️ Project Structure

```
nRF52840-DK-rust/
├── src/
│   ├── main.rs           # Embassy executor and task spawning
│   ├── gpio_tasks.rs     # Async GPIO handlers (LEDs, buttons)
│   └── ble_task.rs       # BLE GATT server implementation
├── .cargo/config.toml    # Cargo configuration for nRF52840
├── Cargo.toml           # Dependencies (Embassy, nrf-softdevice)
├── memory.x             # Memory layout with SoftDevice S140
├── Embed.toml           # probe-rs configuration
├── .vscode/             # VS Code debug configuration
│   ├── launch.json      # Debug profiles
│   └── settings.json    # Rust analyzer settings
├── scripts/             # Utility scripts
└── Makefile             # Convenient build commands
```

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

## 📱 BLE Connection

The device advertises as **"nRF52840-DK"** and provides a custom GATT service:

### Service UUID: `6e400001-b5a3-f393-e0a9-e50e24dcca9e`

**Characteristics:**
- **RX** (`6e400002-...`): Write data to device
- **TX** (`6e400003-...`): Read/notify data from device  
- **Status** (`6e400004-...`): Read device status

### Phone Connection Example

Use any BLE scanner app (nRF Connect, BLE Scanner) to:

1. Scan for "nRF52840-DK"
2. Connect to the device
3. Discover services
4. Write data to RX characteristic
5. Enable notifications on TX characteristic

## 🛠️ Development Commands

```bash
# Build debug version
make build

# Build optimized release version
make release

# Flash debug version
make flash

# Flash release version  
make flash-release

# Start RTT debug session
make debug

# Format code
make format

# Run code checks (clippy + format)
make check

# Clean build artifacts
make clean

# Show all available commands
make help
```

## 🔍 Debugging

### RTT Logging
Real-Time Transfer (RTT) provides fast, non-intrusive logging:

```bash
make debug  # Start RTT session
```

### VS Code Integration
1. Install the "probe-rs" VS Code extension
2. Open the project in VS Code
3. Press F5 to start debugging
4. Breakpoints, variable inspection, and RTT logs work seamlessly

### Manual probe-rs Commands
```bash
# Flash and run
probe-rs run --chip nRF52840_xxAA target/thumbv7em-none-eabihf/debug/nrf52840-dk-template

# Attach for debugging
probe-rs attach --chip nRF52840_xxAA
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