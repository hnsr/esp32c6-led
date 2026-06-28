# Agent instructions

Please also read the @README.md for more information.

This is not a vibe-coded project. The primary goal is learning (embedded) Rust
and ESP32-C6 development, not maximizing implementation speed. Avoid generating
code unless the user explicitly asks for it; prefer explanations, references to
existing code, small examples, and focused guidance.

When giving guidance, take into account the user's experience level:

- Some basic and theoretical understanding of machine-level programming (i.e.
  what bytecode, registers, interrupts are, but never wrote any assembly beyond
  "hello world")
- Reasonably comfortable with systems level C programming (so comparisons to C,
  but only when sensible, are welcome)
- Beginner/intermediate at Rust-based development, so some hand-holding here
  is desired (knows what the borrow-checker does, but might not know fully
  how cargo works or how to organize a codebase)
- Beginner at DIY electronics, also some hand-holding here is desired

When providing guidance on implementing new features, explain the low-level
approach and details (i.e. how would you do it without abstraction libraries),
but also provide hints on which libraries/modules/methods exist for speeding up
development. This way the user can both learn the low-level details but also
learn about the right abstraction/library to effectively write real-world and
maintainable code.

## Shape

- Rust 2024, `#![no_std]`, bare-metal target `riscv32imac-unknown-none-elf`.
- Main firmware binary: `src/bin/main.rs`, binary name `esp32c6-hello`.
- `src/lib.rs` is currently only `#![no_std]`.

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

This expands to `espflash flash --monitor --chip esp32c6`;
it requires `espflash` installed and a connected board.

## Firmware Conventions

- Keep firmware code `no_std` compatible; do not introduce `std`.
- Keep `#![no_main]` on the binary and use `#[esp_hal::main]`.
- Keep `esp_bootloader_esp_idf::esp_app_desc!();`; it provides the ESP-IDF bootloader app descriptor.
- The panic handler currently loops forever. Add richer panic/backtrace support only intentionally.
- `build.rs` passes `-Tlinkall.x` and contains linker diagnostics; avoid removing it unless replacing the boot/link setup deliberately.
- Clippy stack frame threshold is 1024 bytes. `main` allows large stack frames locally because embedded examples often allocate buffers there.
