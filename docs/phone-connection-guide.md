# Phone Connection Guide

This guide shows how to connect to your nRF52840-DK from Android and iOS devices using various BLE apps.

## 📱 Recommended BLE Apps

### Android
- **nRF Connect** (Nordic Semiconductor) - Professional grade
- **BLE Scanner** (Bluepixel Technologies) - User-friendly
- **Bluetooth LE Explorer** (Microsoft) - Windows-style UI

### iOS  
- **nRF Connect** (Nordic Semiconductor) - Same as Android
- **BLE Scanner 4.0** (Tim Trense) - Simple interface
- **LightBlue** (Punch Through) - Developer-focused

## 🔍 Connection Steps

### 1. Enable BLE and Location (Android)
```
Settings → Bluetooth → Enable
Settings → Location → Enable (required for BLE scanning)
```

### 2. Scan for Devices
1. Open your BLE app
2. Start scanning
3. Look for **"nRF52840-DK"** in the device list
4. Device will show as "Connectable" with signal strength

### 3. Connect to Device
1. Tap on "nRF52840-DK" 
2. Wait for connection (LED patterns will change)
3. Connection status will show "Connected"

### 4. Discover Services
The app will automatically discover:
- **Nordic UART Service**: `6E400001-B5A3-F393-E0A9-E50E24DCCA9E`
  - **RX Characteristic**: `6E400002-...` (Write)
  - **TX Characteristic**: `6E400003-...` (Read/Notify) 
  - **Status Characteristic**: `6E400004-...` (Read)

## 📤 Sending Data

### Using nRF Connect

1. Navigate to the **Nordic UART Service**
2. Find the **RX characteristic** (`6E400002-...`)
3. Tap the **write** button (pencil icon)
4. Choose data format:
   - **Text**: Type your message
   - **Hex**: Enter hex bytes like `48656C6C6F` (Hello)
   - **Decimal**: Enter decimal values

5. Send the data
6. Check RTT logs on your development machine to see received data

### Example Data to Send
```
Text: "Hello nRF52!"
Hex:  48656C6C6F206E52463532210A  
Decimal: [72, 101, 108, 108, 111, 32, 110, 82, 70, 53, 50, 33]
```

## 📥 Receiving Data

### Enable Notifications

1. Find the **TX characteristic** (`6E400003-...`)
2. Tap the **notification** button (bell icon)  
3. Enable notifications
4. Device will now send data to your phone automatically

### Reading Status

1. Find the **Status characteristic** (`6E400004-...`)
2. Tap the **read** button (eye icon)
3. View current device status

## 🛠️ Custom Phone App Development

### Android Example (Kotlin)
```kotlin
// Connection setup
private val bluetoothAdapter = BluetoothAdapter.getDefaultAdapter()
private var bluetoothGatt: BluetoothGatt? = null

// Service UUIDs
companion object {
    val UART_SERVICE_UUID = UUID.fromString("6E400001-B5A3-F393-E0A9-E50E24DCCA9E")
    val UART_RX_UUID = UUID.fromString("6E400002-B5A3-F393-E0A9-E50E24DCCA9E")  
    val UART_TX_UUID = UUID.fromString("6E400003-B5A3-F393-E0A9-E50E24DCCA9E")
}

// Connect to device
fun connectToDevice(device: BluetoothDevice) {
    bluetoothGatt = device.connectGatt(this, false, gattCallback)
}

// GATT callback
private val gattCallback = object : BluetoothGattCallback() {
    override fun onConnectionStateChange(gatt: BluetoothGatt, status: Int, newState: Int) {
        if (newState == BluetoothProfile.STATE_CONNECTED) {
            gatt.discoverServices()
        }
    }
    
    override fun onServicesDiscovered(gatt: BluetoothGatt, status: Int) {
        val service = gatt.getService(UART_SERVICE_UUID)
        val rxCharacteristic = service.getCharacteristic(UART_RX_UUID)
        val txCharacteristic = service.getCharacteristic(UART_TX_UUID)
        
        // Enable TX notifications
        gatt.setCharacteristicNotification(txCharacteristic, true)
    }
}

// Send data  
fun sendData(data: String) {
    val service = bluetoothGatt?.getService(UART_SERVICE_UUID)
    val rxCharacteristic = service?.getCharacteristic(UART_RX_UUID)
    rxCharacteristic?.value = data.toByteArray()
    bluetoothGatt?.writeCharacteristic(rxCharacteristic)
}
```

### iOS Example (Swift)
```swift
import CoreBluetooth

class BLEManager: NSObject, CBCentralManagerDelegate, CBPeripheralDelegate {
    var centralManager: CBCentralManager!
    var peripheral: CBPeripheral?
    
    // Service UUIDs
    let uartServiceUUID = CBUUID(string: "6E400001-B5A3-F393-E0A9-E50E24DCCA9E")
    let rxCharUUID = CBUUID(string: "6E400002-B5A3-F393-E0A9-E50E24DCCA9E")
    let txCharUUID = CBUUID(string: "6E400003-B5A3-F393-E0A9-E50E24DCCA9E")
    
    override init() {
        super.init()
        centralManager = CBCentralManager(delegate: self, queue: nil)
    }
    
    func centralManagerDidUpdateState(_ central: CBCentralManager) {
        if central.state == .poweredOn {
            centralManager.scanForPeripherals(withServices: [uartServiceUUID])
        }
    }
    
    func centralManager(_ central: CBCentralManager, didDiscover peripheral: CBPeripheral, 
                       advertisementData: [String : Any], rssi RSSI: NSNumber) {
        if peripheral.name == "nRF52840-DK" {
            self.peripheral = peripheral
            centralManager.connect(peripheral)
        }
    }
    
    func centralManager(_ central: CBCentralManager, didConnect peripheral: CBPeripheral) {
        peripheral.delegate = self
        peripheral.discoverServices([uartServiceUUID])
    }
    
    func sendData(_ data: String) {
        guard let peripheral = peripheral,
              let service = peripheral.services?.first(where: { $0.uuid == uartServiceUUID }),
              let rxChar = service.characteristics?.first(where: { $0.uuid == rxCharUUID }) else {
            return
        }
        
        let dataToSend = data.data(using: .utf8)!
        peripheral.writeValue(dataToSend, for: rxChar, type: .withResponse)
    }
}
```

## 🔧 Testing Your Connection

### Quick Test Sequence

1. **Flash the nRF52840-DK** with the Embassy template
2. **Start RTT logging**: `make debug`
3. **Connect phone** using BLE app
4. **Send test message**: "Hello from phone!"
5. **Check RTT output** for received message
6. **Verify LEDs** respond to button presses

### Expected RTT Output
```
INFO  nRF52840-DK Embassy Template Starting!
INFO  Starting heartbeat task  
INFO  Starting button handler task
INFO  Starting LED pattern task
INFO  Starting BLE task
INFO  BLE SoftDevice initialized
INFO  Starting BLE advertising
INFO  BLE device connected
INFO  Received data: [72, 101, 108, 108, 111, 32, 102, 114, 111, 109, 32, 112, 104, 111, 110, 101, 33]
```

## 🚨 Troubleshooting

### Connection Issues
- **Device not found**: Check if device is advertising (RTT logs)
- **Connection fails**: Reset board and try again  
- **Service discovery fails**: Ensure SoftDevice S140 is properly initialized

### Data Transfer Issues
- **Can't write to RX**: Check characteristic permissions
- **No notifications**: Verify TX characteristic notifications are enabled
- **Garbled data**: Check text encoding (UTF-8 recommended)

### Performance Tips
- **Connection interval**: Default 20ms works well for most apps
- **MTU size**: Template supports up to 128 bytes per packet
- **Throughput**: Expect ~1-2 KB/s depending on phone and connection parameters

## 📚 Further Reading

- [Bluetooth SIG GATT Specifications](https://www.bluetooth.com/specifications/gatt/)
- [Nordic nRF Connect Documentation](https://www.nordicsemi.com/Products/Development-tools/nrf-connect-for-mobile)
- [Embassy BLE Examples](https://github.com/embassy-rs/embassy/tree/master/examples/nrf52840)

---

Happy BLE development! 📱🦀