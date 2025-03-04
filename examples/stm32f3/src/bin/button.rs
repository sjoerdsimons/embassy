#![no_std]
#![no_main]
#![feature(type_alias_impl_trait)]

#[path = "../example_common.rs"]
mod example_common;
use cortex_m_rt::entry;
use embassy_stm32::gpio::{Input, Level, Output, Pull, Speed};
use embedded_hal::digital::v2::{InputPin, OutputPin};
use example_common::*;

#[entry]
fn main() -> ! {
    info!("Hello World!");

    let p = embassy_stm32::init(Default::default());

    let button = Input::new(p.PA0, Pull::Down);
    let mut led1 = Output::new(p.PE9, Level::High, Speed::Low);
    let mut led2 = Output::new(p.PE15, Level::High, Speed::Low);

    loop {
        if unwrap!(button.is_high()) {
            info!("high");
            unwrap!(led1.set_high());
            unwrap!(led2.set_low());
        } else {
            info!("low");
            unwrap!(led1.set_low());
            unwrap!(led2.set_high());
        }
    }
}
