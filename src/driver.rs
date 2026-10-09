use crate::color::Rgbw;
use crate::math::unit_f32_to_u8;
use esp_hal::Blocking;
use esp_hal::gpio::Level;
use esp_hal::peripherals::{Peripherals, GPIO8, RMT};
use esp_hal::rmt::{Channel, PulseCode, Rmt, Tx, TxChannelConfig, TxChannelCreator};
use esp_hal::time::Rate;

const MAX_LEDS: usize = 100;

pub trait Driver {
    fn setup(&mut self, _peripherals: Peripherals) {}

    fn begin_frame(&mut self) {}

    fn write_led(&mut self, index: usize, color: Rgbw);

    fn end_frame(&mut self) {}

    fn teardown(&mut self) {}
}

// todo: Move to submodule
// todo: add driver config (GPIO)

pub struct Ws2812RmtDriver<'a> {
    buffer: [PulseCode; MAX_LEDS * 32 + 1],
    // Needs to be an Option, because we need to move the Channel value out while transmmitting.
    channel: Option<Channel<'a, Blocking, Tx>>,
    max_index: usize,
}

const WS2812_ZERO: PulseCode = PulseCode::new(Level::High, 24, Level::Low, 76);
const WS2812_ONE: PulseCode = PulseCode::new(Level::High, 48, Level::Low, 52);

impl<'a> Ws2812RmtDriver<'a> {
    pub fn new(rmt: RMT<'a>, gpio: GPIO8<'a>) -> Self {
        // The integrated WS2812B-compatible RGB LED is connected to GPIO 8. At an
        // 80 MHz RMT clock, one tick is 12.5 ns; each encoded bit totals 100 ticks.
        let rmt = Rmt::new(rmt, Rate::from_mhz(80)).unwrap();
        let tx_config = TxChannelConfig::default()
            .with_clk_divider(1)
            .with_idle_output_level(Level::Low)
            .with_idle_output(true);
        let channel = rmt
            .channel0
            .configure_tx(&tx_config)
            .unwrap()
            .with_pin(gpio);

        Self {
            buffer: [PulseCode::end_marker(); MAX_LEDS * 32 + 1],
            channel: Some(channel),
            max_index: 0,
        }
    }
}

impl Driver for Ws2812RmtDriver<'_> {
    fn write_led(&mut self, index: usize, color: Rgbw) {
        // Write single color to buffer.

        assert!(index < MAX_LEDS);

        let slice = &mut self.buffer[index * 32..(index + 1) * 32];
        let pulses = get_pulsecodes_for_color(
            unit_f32_to_u8(color.red),
            unit_f32_to_u8(color.green),
            unit_f32_to_u8(color.blue),
            unit_f32_to_u8(color.white),
        );
        slice.copy_from_slice(&pulses);
        self.max_index = index;
    }

    fn end_frame(&mut self) {
        // Write end-marker to buffer and transmit using RMT channel.
        let end_marker_index = (self.max_index + 1) * 32;
        self.buffer[end_marker_index] = PulseCode::end_marker();

        // We have to take() self.channel here so it can be moved into transmit(), after which we moved it back in
        let channel = self.channel.take().unwrap();
        let channel = channel
            .transmit(&self.buffer[..end_marker_index + 1])
            .unwrap()
            // fixme: this blocks the cooperative scheduler and delays handling of zigbee requests
            .wait()
            .unwrap();
        self.channel = Some(channel);

        self.max_index = 0;
    }
}

fn get_pulsecodes_for_color(red: u8, green: u8, blue: u8, white: u8) -> [PulseCode; 32] {
    let color =
        ((green as u32) << 24) | ((red as u32) << 16) | ((blue as u32) << 8) | (white as u32);

    let mut pulses = [PulseCode::end_marker(); 32];

    for (bit, pulse) in pulses.iter_mut().enumerate() {
        let mask = 1 << (31 - bit);
        *pulse = if color & mask == 0 {
            WS2812_ZERO
        } else {
            WS2812_ONE
        };
    }

    pulses
}
