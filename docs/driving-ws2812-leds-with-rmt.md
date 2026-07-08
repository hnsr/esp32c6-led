# Driving WS2812 LEDs with RMT

WS2812-compatible RGB LEDs are controlled through a single digital data pin.
Red, green, and blue are not connected to separate GPIO pins. Instead, the
controller sends a precisely timed sequence of 24 bits for each LED.

## Why use RMT?

RMT is the ESP32's **Remote Control peripheral**. Although originally designed
for infrared remote-control signals, it is useful as a general-purpose hardware
pulse generator.

Software could theoretically drive a WS2812 by repeatedly changing a GPIO and
waiting for a few hundred nanoseconds. In practice, instruction timing,
interrupts, and compiler optimizations make such "bit banging" fragile. With
RMT, software prepares a list of pulse descriptions and the peripheral emits
the waveform with hardware-controlled timing.

The data path is:

```text
PulseCode buffer -> RMT transmit channel -> GPIO matrix -> GPIO pin -> LED
```

## Clocks and ticks

A peripheral clock provides evenly spaced timing events called *ticks*. At an
80 MHz RMT clock, there are 80 million ticks per second, so one tick lasts:

```text
1 / 80,000,000 seconds = 12.5 ns
```

The channel's clock divider can make its effective tick longer. With a divider
of 1, an RMT tick remains 12.5 ns. With a divider of 2, it would be 25 ns.

The CPU clock and RMT clock serve different purposes. The CPU clock determines
how quickly instructions execute; the RMT clock determines the timing
resolution of the generated waveform. Once started, an RMT transmission does
not depend on the CPU executing each signal transition at the correct instant.

## WS2812 bit encoding

A WS2812 bit lasts approximately 1.25 microseconds. Both zero and one begin
high and end low; the duration of each phase identifies the bit:

```text
0 bit:  high for about 400 ns, then low for about 850 ns
1 bit:  high for about 800 ns, then low for about 450 ns
```

At 80 MHz, suitable pulse descriptions are:

```rust
use esp_hal::gpio::Level;
use esp_hal::rmt::PulseCode;

const WS2812_ZERO: PulseCode =
    PulseCode::new(Level::High, 32, Level::Low, 68);
const WS2812_ONE: PulseCode =
    PulseCode::new(Level::High, 64, Level::Low, 36);
```

Each `PulseCode` contains two consecutive output phases. Its arguments describe
the first level and duration, followed by the second level and duration. Thus,
one `PulseCode` conveniently represents one complete WS2812 data bit:

| Value | High phase | Low phase | Total |
|---|---:|---:|---:|
| `0` | 32 ticks = 400 ns | 68 ticks = 850 ns | 1.25 us |
| `1` | 64 ticks = 800 ns | 36 ticks = 450 ns | 1.25 us |

## Color data

One LED consumes 24 bits, most-significant bit first. WS2812 devices normally
expect the bytes in **green, red, blue (GRB)** order:

```text
GGGGGGGG RRRRRRRR BBBBBBBB
```

Each color byte is an intensity from 0 (off) to 255 (maximum). Software converts
each of the 24 logical bits into either `WS2812_ZERO` or `WS2812_ONE`. A final
`PulseCode::end_marker()` tells the RMT peripheral where transmission stops.

After the data, the GPIO must remain low for the LED's required reset/latch
period. During this interval, the LED accepts the received bits as its new
color. Consult the particular LED's datasheet for its timing tolerances and
minimum reset duration.

## Configuring an RMT transmit channel

With `esp-hal`, the peripheral and a transmit channel can be configured as
follows:

```rust
use esp_hal::gpio::Level;
use esp_hal::rmt::{Rmt, TxChannelConfig, TxChannelCreator};
use esp_hal::time::Rate;

let rmt = Rmt::new(peripherals.RMT, Rate::from_mhz(80)).unwrap();

let tx_config = TxChannelConfig::default()
    .with_clk_divider(1)
    .with_idle_output_level(Level::Low)
    .with_idle_output(true);

let channel = rmt
    .channel0
    .configure_tx(&tx_config)
    .unwrap()
    .with_pin(peripherals.GPIO8);
```

The important steps are:

1. `Rmt::new` configures the RMT peripheral's clock.
2. `configure_tx` changes a suitable RMT channel into a transmitter.
3. The divider defines the duration of a channel tick.
4. The idle settings actively hold the output low after transmission, providing
   the state needed for the WS2812 reset/latch period.
5. `with_pin` routes the channel's output through the ESP32 GPIO matrix to the
   selected pin.

## Transmitting pulse data

Given a slice containing encoded pulses and an end marker, blocking transmission
looks like this:

```rust
channel = channel
    .transmit(&pulses)
    .unwrap()
    .wait()
    .unwrap();
```

The channel is deliberately consumed by `transmit`. While the transaction is in
progress, Rust therefore prevents other code from reusing or reconfiguring the
same hardware channel. Calling `wait` drives the blocking transfer to completion
and returns ownership of the channel so it can be used again.

This ownership flow mirrors the actual hardware state:

```text
idle channel -> active transaction -> completed channel
```

For pulse sequences larger than the RMT channel's internal memory, `wait` also
allows the HAL to continue feeding data to the peripheral while the earlier
pulses are being transmitted.

## Abstraction choices

Using `esp-hal::rmt` directly exposes the useful low-level concepts: clock
selection, tick counts, pulse memory, GPIO routing, and transfer ownership. A
higher-level smart-LED or WS2812 driver can handle color ordering and pulse
encoding automatically, while normally relying on an RMT- or SPI-based backend
for the same timing-sensitive output.
