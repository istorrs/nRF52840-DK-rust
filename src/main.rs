#![no_std]
#![no_main]

use defmt::*;
use embassy_executor::Spawner;
use embassy_nrf::gpio::{Input, Level, Output, OutputDrive, Pull};
use embassy_time::{Duration, Timer};
use {defmt_rtt as _, panic_halt as _};

fn rtt_flush() {
    // Small delay to ensure RTT buffer is flushed
    for _ in 0..1000 {
        core::hint::spin_loop();
    }
}

// Simple debug macro
macro_rules! debug_step {
    ($($arg:tt)*) => {
        info!($($arg)*);
        rtt_flush();
    };
}

mod gpio_tasks;
use gpio_tasks::*;

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    // Early RTT test
    defmt::flush();
    info!("=== nRF52840-DK GPIO Template Starting ===");

    // Initialize Embassy
    debug_step!("Step 1: Initializing Embassy...");
    let mut config = embassy_nrf::config::Config::default();
    config.gpiote_interrupt_priority = embassy_nrf::interrupt::Priority::P2;
    config.time_interrupt_priority = embassy_nrf::interrupt::Priority::P2;
    let p = embassy_nrf::init(config);
    info!("✅ Embassy initialized successfully");

    // Configure GPIO pins
    debug_step!("Step 2: Configuring GPIO pins...");
    let led1 = Output::new(p.P0_13, Level::High, OutputDrive::Standard);
    let led2 = Output::new(p.P0_14, Level::High, OutputDrive::Standard);
    let led3 = Output::new(p.P0_15, Level::High, OutputDrive::Standard);
    let led4 = Output::new(p.P0_16, Level::High, OutputDrive::Standard);
    let btn1 = Input::new(p.P0_11, Pull::Up);
    let _btn2 = Input::new(p.P0_12, Pull::Up);
    let _btn3 = Input::new(p.P0_24, Pull::Up);
    let _btn4 = Input::new(p.P0_25, Pull::Up);
    debug_step!("✅ GPIO pins configured");

    // Spawn async tasks
    debug_step!("Step 3: Spawning GPIO tasks...");
    unwrap!(spawner.spawn(heartbeat_task(led1)));
    unwrap!(spawner.spawn(button_handler_task(btn1, led2)));
    unwrap!(spawner.spawn(led_pattern_task(led3, led4)));
    info!("✅ All GPIO tasks spawned successfully");

    info!("All systems operational - GPIO + RTT working!");

    // Main loop with periodic status
    loop {
        info!("Main loop: GPIO + RTT working perfectly");
        rtt_flush();
        Timer::after(Duration::from_millis(5000)).await;
    }
}
