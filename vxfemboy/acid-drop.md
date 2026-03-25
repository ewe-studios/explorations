# Acid-Drop - T-Deck IRC Firmware

**Source:** `/home/darkvoid/Boxxed/@formulas/src.rust/vxfemboy/acid-drop/`

---

## Overview

**Acid-Drop** is a custom firmware for the **LilyGo T-Deck** (ESP32-S3 based handheld device) that provides an IRC client with a graphical interface. Unlike the other vxfemboy projects, this is written in C++ using the Arduino/PlatformIO framework rather than Rust.

### What It Does

- Connects to WiFi networks (with scanning and selection UI)
- Connects to IRC servers (with TLS support)
- Provides a graphical IRC client on the T-Deck's display
- Supports IRC commands and 99 color codes
- Stores preferences in non-volatile memory
- Plays notification sounds through the built-in speaker

### Hardware Target

**LilyGo T-Deck** with ESP32-S3FN16R8:
- ESP32-S3 dual-core Xtensa LX7
- 16MB Flash, 8MB PSRAM
- Built-in keyboard, trackball, and display
- I2S audio, SD card, LoRa capable

---

## Architecture

### Component Diagram

```
┌─────────────────────────────────────────────────────────────┐
│                      Acid-Drop Firmware                      │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐      │
│  │   Display    │  │    Input     │  │   Network    │      │
│  │   (LVGL)     │  │  (Keyboard)  │  │   (WiFi)     │      │
│  └──────┬───────┘  └──────┬───────┘  └──────┬───────┘      │
│         │                 │                 │               │
│         │                 │                 │               │
│         ▼                 ▼                 ▼               │
│  ┌──────────────────────────────────────────────────────┐  │
│  │                   Main Loop                           │  │
│  │  ┌─────────────────────────────────────────────────┐ │  │
│  │  │              IRC Client Core                     │ │  │
│  │  │   - Connect/Reconnect                           │ │  │
│  │  │   - Parse incoming messages                     │ │  │
│  │  │   - Handle PING/PONG                            │ │  │
│  │  │   - Send commands                               │ │  │
│  │  └─────────────────────────────────────────────────┘ │  │
│  └──────────────────────────────────────────────────────┘  │
│         │                 │                 │               │
│         ▼                 ▼                 ▼               │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐      │
│  │   Speaker    │  │   Storage    │  │   Power      │      │
│  │  (I2S Audio) │  │(Preferences) │  │  Management  │      │
│  └──────────────┘  └──────────────┘  └──────────────┘      │
│                                                              │
└─────────────────────────────────────────────────────────────┘
```

### Module Structure

```
src/
├── main.ino          # Entry point, setup()/loop()
├── Display.h/cpp     # LVGL display handling
├── IRC.h/cpp         # IRC protocol handling
├── Network.h/cpp     # WiFi management
├── Speaker.h/cpp     # Audio notifications
├── Storage.h/cpp     # SD card (commented out)
├── Lora.h/cpp        # LoRa radio (commented out)
├── Gotify.h/cpp      # Gotify notifications
├── Utilities.h/cpp   # Helper functions
├── pins.h            # Pin definitions
└── bootScreen.h      # Boot screen XBM bitmap
```

---

## Key Implementation Details

### 1. Main Loop Architecture

The firmware uses the Arduino `setup()`/`loop()` pattern:

```cpp
void setup() {
    Serial.begin(115200);
    pinMode(BOARD_POWERON, OUTPUT);
    digitalWrite(BOARD_POWERON, HIGH);

    Wire.begin(BOARD_I2C_SDA, BOARD_I2C_SCL);
    setupScreen();
    loadPreferences();
    initializeNetwork();
    setupI2S();
    playRTTTL(rtttl_boot);

    // Connect to WiFi or scan
    if (wifi_ssid.length() > 0) {
        connectToWiFi(wifi_ssid, wifi_password);
    } else {
        scanWiFiNetworks();
    }
}

void loop() {
    if (infoScreen) {
        // Show info screen for 10 seconds
    } else if (configScreen) {
        // Show config screen
    } else if (wifi_ssid.length() == 0) {
        // Handle WiFi selection
        char incoming = getKeyboardInput();
        handleWiFiSelection(incoming);
    } else {
        // Normal IRC operation
        if (client && client->connected()) {
            handleIRC();
        } else {
            // Reconnect logic
        }

        // Handle keyboard input for IRC
        char incoming = getKeyboardInput();
        handleKeyboardInput(incoming);

        // Screen timeout on inactivity
        if (millis() - lastActivityTime > INACTIVITY_TIMEOUT)
            turnOffScreen();
    }
}
```

### 2. IRC Protocol Handling

```cpp
void handleIRC() {
    while (client->available()) {
        String line = client->readStringUntil('\n');

        // Parse IRC prefix and command
        int firstSpace = line.indexOf(' ');
        int secondSpace = line.indexOf(' ', firstSpace + 1);

        if (command == "001") {
            // RPL_WELCOME - server ready
            joinChannelTime = millis() + 2500;
            readyToJoinChannel = true;
        }

        // Auto-respond to PING
        if (line.startsWith("PING")) {
            String pingResponse = "PONG " + line.substring(
                line.indexOf(' ') + 1);
            sendIRC(pingResponse);
        } else {
            parseAndDisplay(line);
            lastActivityTime = millis();
        }
    }
}

void sendIRC(String command) {
    // RFC 1459 limits lines to 512 bytes including CRLF
    if (command.length() > 510) {
        Serial.println("Failed to send: Command too long");
        return;
    }

    if (client->connected()) {
        client->println(command);
    }
}
```

### 3. WiFi Network Scanning

```cpp
void scanWiFiNetworks() {
    wifiNetworks.clear();
    int n = WiFi.scanNetworks();

    for (int i = 0; i < n && i < 100; i++) {
        WiFiNetwork net;
        net.index = i + 1;
        net.channel = WiFi.channel(i);
        net.rssi = WiFi.RSSI(i);
        net.encryption = (WiFi.encryptionType(i) == WIFI_AUTH_OPEN)
                         ? "Open" : "Secured";
        net.ssid = WiFi.SSID(i).substring(0, 32);
        wifiNetworks.push_back(net);
    }

    displayWiFiNetworks();
}

void updateSelectedNetwork(int delta) {
    int newIndex = selectedNetworkIndex + delta;
    if (newIndex >= 0 && newIndex < wifiNetworks.size()) {
        selectedNetworkIndex = newIndex;
        displayWiFiNetworks();
    }
}
```

### 4. MAC Address Randomization

```cpp
void randomizeMacAddress() {
    uint8_t new_mac[6];
    for (int i = 0; i < 6; ++i)
        new_mac[i] = random(0x00, 0xFF);

    esp_wifi_set_mac(WIFI_IF_STA, new_mac);
}
```

### 5. TLS Connection Support

```cpp
bool connectToIRC() {
    if (irc_tls) {
        client = new WiFiClientSecure();
        static_cast<WiFiClientSecure*>(client)->setInsecure();
        return static_cast<WiFiClientSecure*>(client)->connect(
            irc_server.c_str(), irc_port);
    } else {
        client = new WiFiClient();
        return client->connect(irc_server.c_str(), irc_port);
    }
}
```

---

## Dependencies

### PlatformIO Configuration

```ini
[platformio]
default_envs = T-Deck
src_dir = src

[env:T-Deck]
platform = espressif32@6.3.0
board = esp32s3box
framework = arduino
upload_speed = 921600
monitor_speed = 115200
board_build.partitions = default_16MB.csv
build_flags =
    -DBOARD_HAS_PSRAM
    -DARDUINO_USB_CDC_ON_BOOT=1
    -DDISABLE_ALL_LIBRARY_WARNINGS
lib_deps =
    ArduinoWebsockets
    Wireguard-ESP32
    earlephilhower/ESP8266Audio
```

### Key Libraries

| Library | Purpose |
|---------|---------|
| ArduinoWebsockets | WebSocket communication |
| Wireguard-ESP32 | VPN tunnel support |
| ESP8266Audio | Audio playback (I2S) |
| LVGL | Graphics library |

---

## Features Roadmap

### Device Functionality

- [x] Screen timeout on inactivity (30 seconds)
- [ ] Keyboard backlight timeout
- [ ] Trackball support
- [x] Speaker support (boot sounds, mention sounds)
- [ ] GPS support
- [ ] LoRa support
- [ ] BLE support
- [ ] SD card support

### Features

- [x] LVGL UI
- [x] WiFi scanning & selection
- [x] Saved WiFi profiles
- [ ] WiFi Hotspot
- [ ] Notifications window
- [x] Status bar (Time, WiFi, Battery, Notifications)
- [ ] Screensaver
- [x] Serial debug logs

### IRC Features

- [x] `/raw` command
- [ ] Message backlog (last 200 messages)
- [ ] Multi-buffer support (`/join`, `/part`, `/0`, `/1`)
- [ ] Status window for raw IRC lines
- [ ] Hilight monitor buffer
- [x] 99 color support
- [ ] `/pm` support
- [ ] NickServ integration

---

## IRC Commands

| Command | Description |
|---------|-------------|
| `/info` | Show hardware information |
| `/me <message>` | Send ACTION message |
| `/nick <new>` | Change IRC nickname |
| `/raw <data>` | Send raw IRC data |

---

## Flashing Instructions

### Using VS Code + PlatformIO

1. Add user to `dialout` group: `sudo gpasswd -a $USER dialout`
2. Install PlatformIO VS Code extension
3. Hold trackball, turn on device, plug in USB
4. Press F1 → `PlatformIO: Build`
5. Press F1 → `PlatformIO: Upload`
6. Press RST button

### Using esptool

```bash
pip install esptool
esptool.py --chip esp32-s3 --port /dev/ttyUSB0 \
    --baud 115200 write_flash -z 0x1000 firmware.bin
```

---

## Debugging

### Serial Debug

```bash
apt-get install screen
screen /dev/ttyAMC0 9600  # or /dev/ttyUSB0
```

### Debug Output

The firmware outputs debug information via Serial:
- WiFi connection status
- IRC messages (incoming/outgoing)
- Network scan results
- Preference storage operations

---

## Comparison to Rust Implementation

If this were implemented in Rust, it would use:

### Potential Rust Stack

```toml
[dependencies]
esp-idf-svc = { version = "0.48", features = ["binstart"] }
esp-idf-hal = "0.44"
embedded-graphics = "0.8"
lvgl = "0.8"
async-io = "2.0"
embedded-svc = { version = "0.27", features = ["async"] }
```

### Key Differences

| Aspect | Arduino (C++) | Rust (esp-idf) |
|--------|---------------|----------------|
| Memory Safety | Manual | Guaranteed |
| Build System | PlatformIO | Cargo + esp-idf |
| Async Support | Limited | Full async/await |
| Error Handling | Return codes | Result<T, E> |
| Package Manager | Arduino Libraries | crates.io |

---

## Interesting Implementation Notes

### 1. Timing Constants

```cpp
const unsigned long STATUS_UPDATE_INTERVAL = 15000; // 15 seconds
const unsigned long INACTIVITY_TIMEOUT = 30000;     // 30 seconds
```

### 2. Preference Storage

```cpp
preferences.begin("config", false);
preferences.putString("wifi_ssid", ssid);
preferences.putString("wifi_password", password);
preferences.end();
```

### 3. RTTTL Audio Playback

```cpp
const char* rtttl_boot = "TakeOnMe:d=4,o=4,b=500:8f#5,8f#5,8f#5,8d5,8p,8b...";
playRTTTL(rtttl_boot);
```

---

## Files

- **Main Entry:** `/home/darkvoid/Boxxed/@formulas/src.rust/vxfemboy/acid-drop/src/main.ino`
- **IRC Handler:** `/home/darkvoid/Boxxed/@formulas/src.rust/vxfemboy/acid-drop/src/IRC.cpp`
- **Network Handler:** `/home/darkvoid/Boxxed/@formulas/src.rust/vxfemboy/acid-drop/src/Network.cpp`
- **Configuration:** `/home/darkvoid/Boxxed/@formulas/src.rust/vxfemboy/acid-drop/platformio.ini`
- **Documentation:** `/home/darkvoid/Boxxed/@formulas/src.rust/vxfemboy/acid-drop/README.md`

---

## Summary

Acid-Drop demonstrates embedded Rust/C++ development patterns:

1. **Event-driven main loop** with state management
2. **Peripheral handling** (display, keyboard, speaker, WiFi)
3. **Protocol implementation** (IRC per RFC 1459)
4. **Power management** (screen timeout)
5. **Persistent storage** (preferences in NVS)

While not written in Rust, it provides insight into embedded development that could be replicated using Rust's esp-idf bindings for ESP32 devices.
