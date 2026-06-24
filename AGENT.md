# Project Agent Notes

This is a small Rust firmware project for learning ESP32-C6 development on an ESP32-C6 Super Mini style board.

## Project Principle

This is not a vibe-coded project. The primary goal is learning embedded Rust and ESP32-C6 development, not maximizing implementation speed. Avoid generating code unless the user explicitly asks for it; prefer explanations, references to existing code, small examples, and focused guidance.

## Shape

- Rust 2024, `#![no_std]`, bare-metal target `riscv32imac-unknown-none-elf`.
- Main firmware binary: `src/bin/main.rs`, binary name `esp32c6-hello`.
- `src/lib.rs` is currently only `#![no_std]`.
- Current behavior: initialize `esp-hal`, then print `Hello World!` every 500 ms.

## Toolchain And Build

- `rust-toolchain.toml` pins stable Rust with `rust-src` and target `riscv32imac-unknown-none-elf`.
- `.cargo/config.toml` sets the default target, `build-std = ["core"]`, and force frame pointers.
- Validate with:

```sh
cargo check
```

- Build with:

```sh
cargo build
cargo build --release
```

- Flash/run uses the configured cargo runner:

```sh
cargo run
```

This expands to `espflash flash --monitor --chip esp32c6`; it requires `espflash` installed and a connected board.

## Important Dependencies

- `esp-hal ~1.1.0` with feature `esp32c6`.
- `esp-bootloader-esp-idf 0.5.0` with feature `esp32c6`.
- `esp-println 0.17.0` with features `esp32c6`, `log-04`.
- `critical-section 1.2.0`.

## Firmware Conventions

- Keep firmware code `no_std` compatible; do not introduce `std`.
- Keep `#![no_main]` on the binary and use `#[esp_hal::main]`.
- Keep `esp_bootloader_esp_idf::esp_app_desc!();`; it provides the ESP-IDF bootloader app descriptor.
- The panic handler currently loops forever. Add richer panic/backtrace support only intentionally.
- `build.rs` passes `-Tlinkall.x` and contains linker diagnostics; avoid removing it unless replacing the boot/link setup deliberately.
- Clippy stack frame threshold is 1024 bytes. `main` allows large stack frames locally because embedded examples often allocate buffers there.

## Hardware Roadmap From README

- Integrated WS2812B LED, probably GPIO 8.
- SK6812 RGBWW LED strip.
- Zigbee control for LED strip behavior.

Prefer small, explicit hardware changes. When adding peripherals, document assumed GPIO pins and timing requirements in code or README.
