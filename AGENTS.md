# Agent instructions

Please also read the @README.md for more information.

Do not create or modify code. The primary goal is learning (embedded) Rust
and ESP32 development, not maximizing implementation speed. Prefer explanations,
references to existing code, small examples, and focused guidance. When providing
longer code examples, generously annotate with comments.

When giving guidance, take into account the user's experience level:

- Some basic and theoretical understanding of machine-level programming (i.e.
  what bytecode, registers, interrupts are, but never wrote any assembly beyond
  "hello world")
- Reasonably comfortable with systems level C programming (so comparisons to C,
  when sensible, are welcome)
- Beginner/intermediate at Rust-based development
- Beginner at DIY electronics.

When providing guidance on implementing new features, explain the low-level
approach and details (i.e. how would you do it without abstraction libraries),
but also provide hints on which libraries/modules/methods exist for speeding up
development. This way the user can both learn the low-level details but also
learn about the right abstraction/library to effectively write real-world and
maintainable code.

## Shape

- Rust 2024, `#![no_std]`, bare-metal target `riscv32imac-unknown-none-elf`.
- Main firmware binary: `src/bin/main.rs`, binary name `esp32c6-led`.
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
