# esp32c6-led

This is a little project to learn rust-based development for an esp32-c6 (super mini)
board, that I want to use for some DIY LED strip projects and connect to my
Philips Hue smart home (Zigbee) network.

## Compiling, flashing, monitoring:

Ensure your user has r/w permission to serial devices:

```shell
# The dialup group name applies to Fedora, check you distro's docs.
sudo usermod -aG dialout "$USER"
```

```shell
# Compile + flashing
cargo run --release

# Flashing previously built binary using sudo:
espflash flash --monitor --chip esp32c6 target/riscv32imac-unknown-none-elf/release/esp32c6-led

# To monitor from boot-up, attach using the below command, then hit CTLR+R to reset the ESP
espflash monitor --chip esp32c6 --port /dev/ttyACM0

# To actually attach to the running firmware without resetting it we can use `picocom`:
picocom --baud 115200 --flow n --lower-rts --raise-dtr --noreset --imap lfcrlf /dev/ttyACM0

# To erase zigbee storage (i.e. if you need to reset network key/state)
espflash erase-region --chip esp32c6 0x3f0000 0x4000 # Double check the range with partitions.csv
```

Prefix with ESP_LOG to build with logging, for example:
```shell
# Log 'warn' level and higher, for our crate and zigbee log 'debug' and higher, but keep zdo at 'info'
# to avoid noisy logging
ESP_LOG='warn,esp32c6_led=debug,zigbee=debug,zigbee_core::zdo=info' cargo run --release
```

## TODO

- [x] Implement Zigbee light control
- [x] Implement a robust, fixed refresh rate rendering loop
- [ ] Implement LED HW config abstraction (diode layout, temperature emulation, GPIO pin selection)
- [ ] Implement Zigbee groups and scenes clusters
- [ ] Implement Zigbee lamp state storage to flash
- [ ] Implement Zigbee transition effects (fade in/out)
- [ ] Implement Zigbee dynamic netweork joining (without hardcoded netwerk PAN)
- [ ] Implement button for: rotating effects (short press), reset zigbee association (long press)
- [ ] Implement more effects, layouts

## HW setup

A level converter is used to convert 3.3V to 5V, as 3.3 is too low for
SK6812/WS2812B addressable LED strips.

- Power supply
  - Typically powered by 2A USB charger connected 5V and GND pins
    - Its GND line is connected to: ESP32-C6 GND pin, level shifter GND, LED strip GND
    - Its 5V line is connected to ESCP32-C6 5V pin, level shifter VB, LED strip 5V in
  - For debugging sometimes powered from laptop USB-C port to on-board USB-C port
    - This actually still powers the LED strip; it seems the 5V pin on the ESP32-C6 will be powered by
      the USC-C port, when connected
    - Amperage should not exceed 1A with this setup, due to physical current limit of the ESP32 
- ESP32-C6 SuperMini
  - Has an WS2812 RGB (not W) LED on GPIO8
  - GPIO8 is wired into level converter A7
  - 3V3 OUT is wired into level converter VA (voltage A)
  - 3V3 OUT is wired into level converter OE (output enable - so it's always enabled)
- Logic level converter (TXS0108E)
  - B7 is wired into LED strip connector DATA line, with a resistor in between (TODO: look up resistance value)
- BTF-Lighting RGBWW (warm white) LED strip (SK6812, WS2812B compatible)
  - LED strip length is 1.27m, 78 LEDs total (60/meter)
  - BTF specs claim theoretical 0.3W power usage per LED, could be more?

### Power usage and limits

When powered through ESP USB-C port, amperage should be limited to 1A. Although this bypasses the voltage regulator
and connects  to the 5V pin, the board may be physically limited to 1A.

Account for about 1A per the first 13-14 LEDs, 2A for 30 LEDs (slightly less ESP/shifter/dark pixel overhead). So to
power all ~76 pixels we need 4.5A-5A (22.5-25W).

References:
- https://kno.wled.ge/advanced/wiring/

## LED rendering

(Work in progress)

Driving the LED strip is implemented as a kind of graphics pipeline with two main pieces which
can have different implementations:

- Layout: determines where each LED is physically located in space
- Effect: determines the color of a specific LED

For determining the layout, different implementations exist for linear, grid, circular
and explicit layouts (and so on).

For effects, you might have a solid color, a pulsating color, or some effect that
combines multiple colors and so on.

Layouts and effects share a common trait but have their own specific configuration.

The renderer is responsible for applying the configured layout + effect and actually
updating the physical LED color.

A global render context holds common state (like elapsed time), utilites (rng)
