# esp32c6-led

This is a little project to learn rust-based development for an esp32-c6 (super mini)
board, that I want to use for some DIY LED strip projects.

- [x] Hello world
- [x] Manipulate integrated WS2812B LED (GPIO 8?)
- [x] Wire up and drive SK6812 RGBWW LED strip
- [ ] Implement full LED strip rendering loop and color control
- [ ] Implement Zigbee for basic LED strip control

## Compiling, flashing, monitoring:

```sh
# Compile + flashing (flashing fails due to permissions)
cargo run --release

# Flashing previously built binary using sudo:
sudo `which espflash` flash --monitor --chip esp32c6 target/riscv32imac-unknown-none-elf/release/esp32c6-led

# Monitor output through serial/JTAG
sudo `which espflash` monitor --chip esp32c6
```

## LED rendering

Ultimately, a series of RGBW pulsecodes need to be constructed and sent off with RMT.

To allow implementing interesting effects in a convenient way, the LED renderer takes callbacks:

- layout: given a LED position, it should return a stable 1/2/3D coordinate
- transform: given a coordinate and time, return a modified coordinate
- shader: given a coordinate and time, return RGBW color

The renderer will take the resulting RGBW colors, construct the pulsecodes and drive the LED strip.