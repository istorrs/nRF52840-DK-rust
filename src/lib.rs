#![no_std]

//! nRF52840-DK Embassy Template Library
//!
//! This library provides reusable components for Embassy-based
//! nRF52840 development including GPIO tasks and BLE functionality.

// pub mod ble_task;  // Disabled for GPIO-only mode
pub mod gpio_tasks;

// CLI interface modules (conditional compilation for cli feature)
#[cfg(feature = "cli")]
pub mod cli;

#[cfg(feature = "cli")]
pub mod mtu;

#[cfg(feature = "cli")]
pub mod meter;
