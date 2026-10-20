use alloc::{boxed::Box, vec};
use crate::color::Rgb;
use crate::math::unit_f32_to_u8;
use esp_hal::Blocking;
use esp_hal::gpio::Level;
use esp_hal::peripherals::{Peripherals, GPIO8, RMT};
use esp_hal::rmt::{Channel, PulseCode, Rmt, Tx, TxChannelConfig, TxChannelCreator};
use esp_hal::time::Rate;
use crate::hw::{DiodeLayout, HardwareConfig};

pub trait Driver {
    fn setup(&mut self, _peripherals: Peripherals) {}

    fn begin_frame(&mut self) {}

    /// When writing colors we use the final RGB value with effect modulation and lamp level baked
    /// in
    fn write_color(&mut self, index: usize, color: Rgb);

    /// When writing a temperature, we use a normalized (0.0 to 1.0) value and intensity is passed
    /// separately
    fn write_temperature(
        &mut self,
        index: usize,
        temperature: f32,
        intensity: f32,
    );

    /// Fixed white appearance, only intensity is provided for dimming.
    fn write_intensity(&mut self, index: usize, intensity: f32);

    fn end_frame(&mut self) {}

    fn teardown(&mut self) {}
}

// todo: Move to submodule
// todo: support dynamic GPIO
// todo: implement correct timings for the various chips
pub struct AddressableLedRmtDriver<'a> {
    buffer: Box<[PulseCode]>,

    // Needs to be an Option, because we need to move the Channel value out while transmitting.
    channel: Option<Channel<'a, Blocking, Tx>>,

    // Keep track how far into the buffer we wrote pulses, so we can write the end marker.
    used_pulses: usize,

    channels_per_pixel: usize,

    hardware_config: &'a HardwareConfig
}

const SK6812_ZERO: PulseCode = PulseCode::new(Level::High, 24, Level::Low, 76);
const SK6812_ONE: PulseCode = PulseCode::new(Level::High, 48, Level::Low, 52);

impl<'a> AddressableLedRmtDriver<'a> {
    pub fn new(hardware_config: &'a HardwareConfig, rmt: RMT<'a>, gpio: GPIO8<'a>) -> Self {
        // The integrated WS2812B-compatible RGB LED is connected to GPIO 8. At an
        // 80 MHz RMT clock, one tick is 12.5 ns; each encoded bit totals 100 ticks.
        let rmt = Rmt::new(rmt, Rate::from_mhz(80)).unwrap();
        let tx_config = TxChannelConfig::default()
            .with_clk_divider(1)
            .with_idle_output_level(Level::Low)
            .with_idle_output(true)
            // Increase number of memory blocks from the default of 1. With only 1 block, we suffer
            // from RMT buffer underrun due to Zigbee radio interrupts, causing LEDs to occasionally
            // glitch (as it will wrap and send stale pulsecodes we didn't update in time).
            // Memsize 4 mitigates this, at least up to ~80 LEDs. But we now claim all (shared) RMT
            // blocks, so if we need more RMT channels or if the issue returns with a larger number
            // of LEDs we should consider SPI with DMA to transfer pulsecodes, which can't be
            // interrupted by the radio.
            .with_memsize(4);
        let channel = rmt
            .channel0
            .configure_tx(&tx_config)
            .unwrap()
            .with_pin(gpio);

        let channels_per_pixel = match hardware_config.diodes {
            DiodeLayout::White { .. } => 1,
            DiodeLayout::WhiteCct { .. } => 2,
            DiodeLayout::Rgb { .. } => 3,
            DiodeLayout::Rgbw { .. } => 4,
            DiodeLayout::RgbCct { .. } => 5,
        };

        // The buffer needs to hold pulses for every LED pixel (= channels * 8, one for every bit),
        // plus one end-marker pulsecode.
        let pulse_count = hardware_config.led_count * channels_per_pixel * 8 + 1;

        // Allocate vector, then convert it into fixed-size Box
        let buffer = vec![PulseCode::end_marker(); pulse_count].into_boxed_slice();

        Self {
            buffer,
            channel: Some(channel),
            used_pulses: 0,
            channels_per_pixel,
            hardware_config
        }
    }

    fn write_channels(&mut self, index: usize, channels: &[u8]) {
        let pulses_per_pixel = channels.len() * 8;
        let start = pulses_per_pixel * index;
        let end = start + pulses_per_pixel;

        assert!(end < self.buffer.len());
        assert_eq!(channels.len(), self.channels_per_pixel);
        assert!(index < self.hardware_config.led_count);

        encode_channels(channels, &mut self.buffer[start..end]);

        self.used_pulses = self.used_pulses.max(end);
    }
}

impl Driver for AddressableLedRmtDriver<'_> {
    fn write_color(&mut self, index: usize, color: Rgb) {
        // Write given RGB color into internal pulse code buffer for given pixel index, taking into
        // account the diode layout
        assert!(index < self.hardware_config.led_count);

        let red = unit_f32_to_u8(color.red);
        let green = unit_f32_to_u8(color.green);
        let blue = unit_f32_to_u8(color.blue);

        match self.hardware_config.diodes {
            DiodeLayout::Rgb { .. } => {
                self.write_channels(index, &[ green, red, blue ]);
            }
            DiodeLayout::Rgbw { .. } => {
                self.write_channels(index, &[ green, red, blue, 0 ]);
            }
            DiodeLayout::RgbCct { .. } => {
                self.write_channels(index, &[ green, red, blue, 0, 0 ]);
            }
            DiodeLayout::White { .. } |
            DiodeLayout::WhiteCct { .. } => {
                unreachable!(
                    "write_color requires RGB diodes, but provided hardware config has none"
                );
            }
        }
    }

    fn write_temperature(&mut self, _index: usize, _temperature: f32, _intensity: f32) {
        // todo: look at HardwareConfig to determine:
        //   - Write to physical CCT diodes
        //   - Write to single white diode + RGB nudging
        //   - Emulate solely using RGB diodes
        todo!()
    }

    fn write_intensity(&mut self, _index: usize, _intensity: f32) {
        // todo: just write the intensity to diodes, but depending on diode layout there
        //       are options on how to do this
        todo!()
    }

    fn end_frame(&mut self) {
        // Write end-marker to buffer and transmit using RMT channel.
        let end_marker_index = self.used_pulses;
        self.buffer[end_marker_index] = PulseCode::end_marker();

        // We have to take() self.channel here so it can be moved into transmit(), after which we moved it back in
        let channel = self.channel.take().unwrap();
        let channel = channel
            .transmit(&self.buffer[..end_marker_index + 1])
            .unwrap()
            // fixme: this blocks the cooperative scheduler and delays handling of zigbee requests,
            //        note that this is at the protocol level, not MAC. Frame acks happen through
            //        interrupts and should be fine.
            .wait()
            .unwrap();
        self.channel = Some(channel);

        self.used_pulses = 0;
    }
}

// Encode given channels (u8) into pulsecode buffer
fn encode_channels(channels: &[u8], output: &mut [PulseCode]) {

    // Ensure output is big enough to write the given number of channels
    assert_eq!(output.len(), channels.len() * 8);

    // Using zip to iterate over each pair of channel byte and output buffer pulse slots
    for (&byte, pulses) in channels.iter().zip(output.chunks_exact_mut(8)) {

        // Iterate over each pulse slot and enumerate into `bit`, so we can use it to extract
        // channel bits
        for (bit, pulse) in pulses.iter_mut().enumerate() {
            // We must send the most significant bit first. We use `bit` to construct the right
            // mask and apply it to `byte` to check if it's set, and write the correct pulse code
            // pattern into the buffer slot.
            let mask = 0x80_u8 >> bit;

            *pulse = if byte & mask == 0 {
                SK6812_ZERO
            } else {
                SK6812_ONE
            };
        }
    }
}
