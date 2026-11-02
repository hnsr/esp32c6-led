use embassy_futures::select::{select, Either};
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::channel::Channel;
use embassy_time::Timer;
use esp_hal::gpio;
use esp_println::println;

#[derive(Debug)]
pub enum ButtonEvent {
    ShortPress,
    LongPress,
}

pub static BUTTON_EVENTS: Channel<CriticalSectionRawMutex, ButtonEvent, 4> =
    Channel::new();

#[embassy_executor::task]
pub async fn button_task(mut button: gpio::Input<'static>) {
    loop {
        // Await a (debounced) press
        button_wait_debounced(&mut button, true).await;

        // Wait for either a de-press or a timer
        let outcome = select(
            button_wait_debounced(&mut button, false),
            Timer::after_millis(3_000)
        ).await;

        match outcome {
            Either::First(()) => {
                println!("Short press");
            }
            Either::Second(()) => {
                println!("Long press");

                // Consume potentially still pending de-press event
                button_wait_debounced(&mut button, false).await
            }
        }
    }
}

async fn button_wait_debounced(button: &mut gpio::Input<'_>, wait_for_pressed: bool) {
    loop {
        if wait_for_pressed {
            button.wait_for_low().await;
        } else {
            button.wait_for_high().await;
        }

        // Wait 15ms for contact to settle
        Timer::after_millis(15).await;

        if button.is_low() == wait_for_pressed {
            return;
        }
    }
}