# esp32c6-hello

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
sudo `which espflash` flash --monitor --chip esp32c6 target/riscv32imac-unknown-none-elf/release/esp32c6-hello

# Monitor output through serial/JTAG
sudo `which espflash` monitor --chip esp32c6
```
