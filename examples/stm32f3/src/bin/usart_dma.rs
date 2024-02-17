#![no_std]
#![no_main]

use core::fmt::Write;

use defmt::*;
use embassy_executor::Spawner;
use embassy_stm32::dma::NoDma;
use embassy_stm32::usart::{Config, Uart};
use embassy_stm32::{bind_interrupts, peripherals, usart};
use heapless::String;
use {defmt_rtt as _, panic_probe as _};

bind_interrupts!(struct Irqs {
    USART1 => usart::InterruptHandler<peripherals::USART1>;
});

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let mut init_config = embassy_stm32::Config::default();
    init_config.rcc.cfgr3.usart1sw = Some(embassy_stm32::pac::rcc::vals::Usart1sw::HSI);
    let p = embassy_stm32::init(init_config);
    info!("Hello World!");

    let config = Config::default();
    let mut usart = Uart::new(p.USART1, p.PE1, p.PE0, Irqs, p.DMA1_CH4, NoDma, config).unwrap();

    for n in 0u32.. {
        let mut s: String<128> = String::new();
        core::write!(&mut s, "Hello DMA World {}!\r\n", n).unwrap();

        unwrap!(usart.write(s.as_bytes()).await);
        info!("wrote DMA");
    }
}
