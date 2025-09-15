use defmt::*;
use embassy_executor::task;
use embassy_nrf::gpio::{Input, Output};
use embassy_time::{Duration, Timer};

#[task]
pub async fn heartbeat_task(mut led: Output<'static>) {
    info!("Starting heartbeat task");
    loop {
        // Blink LED every 500ms to show system is alive
        led.set_low(); // LED on (active low)
        Timer::after(Duration::from_millis(100)).await;
        led.set_high(); // LED off
        Timer::after(Duration::from_millis(400)).await;
    }
}

#[task]
pub async fn button_handler_task(button: Input<'static>, mut led: Output<'static>) {
    info!("Starting button handler task");
    let mut button_pressed = false;

    loop {
        // Check button state (active low)
        let is_pressed = button.is_low();

        if is_pressed && !button_pressed {
            info!("Button pressed!");
            led.set_low(); // Turn on LED
            button_pressed = true;
        } else if !is_pressed && button_pressed {
            info!("Button released!");
            led.set_high(); // Turn off LED
            button_pressed = false;
        }

        // Small delay to debounce
        Timer::after(Duration::from_millis(50)).await;
    }
}

#[task]
pub async fn led_pattern_task(mut led1: Output<'static>, mut led2: Output<'static>) {
    info!("Starting LED pattern task");
    loop {
        // Alternating pattern
        led1.set_low(); // LED1 on
        led2.set_high(); // LED2 off
        Timer::after(Duration::from_millis(1000)).await;

        led1.set_high(); // LED1 off
        led2.set_low(); // LED2 on
        Timer::after(Duration::from_millis(1000)).await;
    }
}
