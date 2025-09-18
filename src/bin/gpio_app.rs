#![no_std]
#![no_main]

use defmt::*;
use embassy_executor::Spawner;
use embassy_nrf::gpio::{Input, Level, Output, OutputDrive, Pull};
use embassy_time::{Duration, Timer};
use {defmt_rtt as _, panic_halt as _};

// GPIO tasks module
mod gpio_tasks {
    use super::*;

    #[embassy_executor::task]
    pub async fn heartbeat_task(mut led: Output<'static>) {
        info!("Starting heartbeat task");
        loop {
            led.set_low();
            Timer::after(Duration::from_millis(100)).await;
            led.set_high();
            Timer::after(Duration::from_millis(900)).await;
        }
    }

    #[embassy_executor::task]
    pub async fn button_handler_task(button: Input<'static>, mut led: Output<'static>) {
        info!("Starting button handler task");
        info!(
            "Button initial state: {}",
            if button.is_low() { "LOW" } else { "HIGH" }
        );

        let mut last_state = button.is_high(); // true when not pressed (pull-up)

        loop {
            let current_state = button.is_high();

            // Button pressed (high to low transition)
            if last_state && !current_state {
                info!("Button pressed!");
                led.set_low(); // Turn on LED (active low)
            }
            // Button released (low to high transition)
            else if !last_state && current_state {
                info!("Button released!");
                led.set_high(); // Turn off LED
            }

            last_state = current_state;

            // Poll every 10ms for responsive button handling
            Timer::after(Duration::from_millis(10)).await;
        }
    }

    #[embassy_executor::task]
    pub async fn led_pattern_task(mut led3: Output<'static>, mut led4: Output<'static>) {
        info!("Starting LED pattern task");
        loop {
            // Pattern: LED3 and LED4 alternating
            led3.set_low();
            led4.set_high();
            Timer::after(Duration::from_millis(300)).await;

            led3.set_high();
            led4.set_low();
            Timer::after(Duration::from_millis(300)).await;
        }
    }
}

use gpio_tasks::*;

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    info!("=== nRF52840-DK SoftDevice-Compatible GPIO App ===");

    // Initialize Embassy (compatible with SoftDevice presence)
    info!("Initializing Embassy...");
    let mut config = embassy_nrf::config::Config::default();
    config.gpiote_interrupt_priority = embassy_nrf::interrupt::Priority::P2;
    config.time_interrupt_priority = embassy_nrf::interrupt::Priority::P2;
    let p = embassy_nrf::init(config);
    info!("✅ Embassy initialized successfully");

    // Configure GPIO pins
    info!("Configuring GPIO pins...");
    let led1 = Output::new(p.P0_13, Level::High, OutputDrive::Standard);
    let led2 = Output::new(p.P0_14, Level::High, OutputDrive::Standard);
    let led3 = Output::new(p.P0_15, Level::High, OutputDrive::Standard);
    let led4 = Output::new(p.P0_16, Level::High, OutputDrive::Standard);
    let btn1 = Input::new(p.P0_11, Pull::Up);
    let _btn2 = Input::new(p.P0_12, Pull::Up);
    let _btn3 = Input::new(p.P0_24, Pull::Up);
    let _btn4 = Input::new(p.P0_25, Pull::Up);
    info!("✅ GPIO pins configured");

    // Spawn async tasks
    info!("Spawning GPIO tasks...");
    unwrap!(spawner.spawn(heartbeat_task(led1)));
    unwrap!(spawner.spawn(button_handler_task(btn1, led2)));
    unwrap!(spawner.spawn(led_pattern_task(led3, led4)));
    info!("✅ All GPIO tasks spawned successfully");

    info!("All systems operational - GPIO working with SoftDevice preserved!");

    // Main loop with periodic status
    loop {
        info!("GPIO app running - SoftDevice preserved, RTT working!");
        Timer::after(Duration::from_millis(5000)).await;
    }
}
