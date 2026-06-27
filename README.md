# esp32c6-hello

This is a little project to learn rust-based development for an esp32-c6 (super mini) board.

- [x] Hello world
- [ ] Manipulate integrated WS2812B LED (GPIO 8?)
- [ ] Wire up and drive SK6812 RGBWW LED strip
- [ ] Implement Zigbee for basic LED strip control

## Compiling, flashing, monitoring:

```sh
# Compile + flashing (flashing fails due to permissions)
cargo run --release

# Flashing previously built binary using sudo:
sudo `which espflash` flash --monitor --chip esp32c6 target/riscv32imac-unknown-none-elf/release/esp32c6-hello

# Monitor output through serial/JTAG
sudo `which espflash` monitor --chip esp32c6
```