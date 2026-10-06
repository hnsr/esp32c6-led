# esp32c6-led

This is a little project to learn rust-based development for an esp32-c6 (super mini)
board, that I want to use for some DIY LED strip projects.

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

## TODO

- [ ] Implement Zigbee light control
- [ ] Implement button for: rotating effects, reset zigbee association (long press)
- [ ] Implement a robust, fixed refresh rate rendering loop
- [ ] Implement more effects, layouts

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