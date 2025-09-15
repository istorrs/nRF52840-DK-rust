#![no_std]
#![no_main]
#![feature(type_alias_impl_trait)]

use defmt::*;
use embassy_executor::Spawner;
use embassy_nrf::gpio::{Input, Level, Output, OutputDrive, Pin, Pull};
use embassy_nrf::{bind_interrupts, peripherals};
use embassy_time::{Duration, Timer};
use {defmt_rtt as _, panic_halt as _};

mod gpio_tasks;
mod ble_task;

use gpio_tasks::*;
use ble_task::*;

bind_interrupts!(struct Irqs {
    POWER_CLOCK => embassy_nrf::power::InterruptHandler;
    RADIO => embassy_nrf::radio::InterruptHandler;
    RTC1 => embassy_nrf::time_driver::InterruptHandler;
    GPIOTE => embassy_nrf::gpiote::InterruptHandler;
    SAADC => embassy_nrf::saadc::InterruptHandler;
});

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    info!("nRF52840-DK Embassy Template Starting!");
    
    let p = embassy_nrf::init(Default::default());
    
    // Configure GPIO pins for nRF52840-DK
    // LEDs: P0.13, P0.14, P0.15, P0.16 (active low)
    let led1 = Output::new(p.P0_13, Level::High, OutputDrive::Standard);
    let led2 = Output::new(p.P0_14, Level::High, OutputDrive::Standard);
    let led3 = Output::new(p.P0_15, Level::High, OutputDrive::Standard);
    let led4 = Output::new(p.P0_16, Level::High, OutputDrive::Standard);
    
    // Buttons: P0.11, P0.12, P0.24, P0.25 (active low with internal pull-up)
    let btn1 = Input::new(p.P0_11, Pull::Up);
    let btn2 = Input::new(p.P0_12, Pull::Up);
    let btn3 = Input::new(p.P0_24, Pull::Up);
    let btn4 = Input::new(p.P0_25, Pull::Up);
    
    // Spawn async tasks
    spawner.spawn(heartbeat_task(led1)).unwrap();
    spawner.spawn(button_handler_task(btn1, led2)).unwrap();
    spawner.spawn(led_pattern_task(led3, led4)).unwrap();
    spawner.spawn(ble_task(p.RADIO, p.TIMER0)).unwrap();
    
    // Main loop - can be used for other tasks or just sleep
    loop {
        info!("Main loop iteration - system running");
        Timer::after(Duration::from_secs(10)).await;
    }
}