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

# Monitor output through serial/JTAG
espflash monitor --chip esp32c6
```

## TODO

- [ ] Fully implement LED rendering system
- [ ] Implement Zigbee control
- [ ] Implement configuration system (web interface?)
- [ ] Implement browser-based mock renderer for effect development 

## LED rendering

(Work in progess)

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