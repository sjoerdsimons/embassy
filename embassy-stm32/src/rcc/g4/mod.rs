use crate::pac;
use crate::peripherals::{self, RCC};
use crate::rcc::{get_freqs, set_freqs, Clocks};
use crate::time::Hertz;
use crate::time::U32Ext;
use core::marker::PhantomData;
use embassy::util::Unborrow;
use embassy_hal_common::unborrow;

/// HSI speed
pub const HSI_FREQ: u32 = 16_000_000;

/// LSI speed
pub const LSI_FREQ: u32 = 32_000;

/// System clock mux source
#[derive(Clone, Copy)]
pub enum ClockSrc {
    HSE(Hertz),
    HSI16,
}

/// AHB prescaler
#[derive(Clone, Copy, PartialEq)]
pub enum AHBPrescaler {
    NotDivided,
    Div2,
    Div4,
    Div8,
    Div16,
    Div64,
    Div128,
    Div256,
    Div512,
}

/// APB prescaler
#[derive(Clone, Copy)]
pub enum APBPrescaler {
    NotDivided,
    Div2,
    Div4,
    Div8,
    Div16,
}

impl Into<u8> for APBPrescaler {
    fn into(self) -> u8 {
        match self {
            APBPrescaler::NotDivided => 1,
            APBPrescaler::Div2 => 0x04,
            APBPrescaler::Div4 => 0x05,
            APBPrescaler::Div8 => 0x06,
            APBPrescaler::Div16 => 0x07,
        }
    }
}

impl Into<u8> for AHBPrescaler {
    fn into(self) -> u8 {
        match self {
            AHBPrescaler::NotDivided => 1,
            AHBPrescaler::Div2 => 0x08,
            AHBPrescaler::Div4 => 0x09,
            AHBPrescaler::Div8 => 0x0a,
            AHBPrescaler::Div16 => 0x0b,
            AHBPrescaler::Div64 => 0x0c,
            AHBPrescaler::Div128 => 0x0d,
            AHBPrescaler::Div256 => 0x0e,
            AHBPrescaler::Div512 => 0x0f,
        }
    }
}

/// Clocks configutation
pub struct Config {
    mux: ClockSrc,
    ahb_pre: AHBPrescaler,
    apb1_pre: APBPrescaler,
    apb2_pre: APBPrescaler,
    low_power_run: bool,
}

impl Default for Config {
    #[inline]
    fn default() -> Config {
        Config {
            mux: ClockSrc::HSI16,
            ahb_pre: AHBPrescaler::NotDivided,
            apb1_pre: APBPrescaler::NotDivided,
            apb2_pre: APBPrescaler::NotDivided,
            low_power_run: false,
        }
    }
}

impl Config {
    #[inline]
    pub fn clock_src(mut self, mux: ClockSrc) -> Self {
        self.mux = mux;
        self
    }

    #[inline]
    pub fn ahb_pre(mut self, pre: AHBPrescaler) -> Self {
        self.ahb_pre = pre;
        self
    }

    #[inline]
    pub fn apb1_pre(mut self, pre: APBPrescaler) -> Self {
        self.apb1_pre = pre;
        self
    }

    #[inline]
    pub fn apb2_pre(mut self, pre: APBPrescaler) -> Self {
        self.apb2_pre = pre;
        self
    }

    #[inline]
    pub fn low_power_run(mut self, on: bool) -> Self {
        self.low_power_run = on;
        self
    }
}

/// RCC peripheral
pub struct Rcc<'d> {
    _rb: peripherals::RCC,
    phantom: PhantomData<&'d mut peripherals::RCC>,
}

impl<'d> Rcc<'d> {
    pub fn new(rcc: impl Unborrow<Target = peripherals::RCC> + 'd) -> Self {
        unborrow!(rcc);
        Self {
            _rb: rcc,
            phantom: PhantomData,
        }
    }

    // Safety: RCC init must have been called
    pub fn clocks(&self) -> &'static Clocks {
        unsafe { get_freqs() }
    }
}

/// Extension trait that freezes the `RCC` peripheral with provided clocks configuration
pub trait RccExt {
    fn freeze(self, config: Config) -> Clocks;
}

impl RccExt for RCC {
    #[inline]
    fn freeze(self, cfgr: Config) -> Clocks {
        let rcc = pac::RCC;
        let (sys_clk, sw) = match cfgr.mux {
            ClockSrc::HSI16 => {
                // Enable HSI16
                unsafe {
                    rcc.cr().write(|w| w.set_hsion(true));
                    while !rcc.cr().read().hsirdy() {}
                }

                (HSI_FREQ, 0x01)
            }
            ClockSrc::HSE(freq) => {
                // Enable HSE
                unsafe {
                    rcc.cr().write(|w| w.set_hseon(true));
                    while !rcc.cr().read().hserdy() {}
                }

                (freq.0, 0x02)
            }
        };

        unsafe {
            rcc.cfgr().modify(|w| {
                w.set_sw(sw.into());
                w.set_hpre(cfgr.ahb_pre.into());
                w.set_ppre1(cfgr.apb1_pre.into());
                w.set_ppre2(cfgr.apb2_pre.into());
            });
        }

        let ahb_freq: u32 = match cfgr.ahb_pre {
            AHBPrescaler::NotDivided => sys_clk,
            pre => {
                let pre: u8 = pre.into();
                let pre = 1 << (pre as u32 - 7);
                sys_clk / pre
            }
        };

        let (apb1_freq, apb1_tim_freq) = match cfgr.apb1_pre {
            APBPrescaler::NotDivided => (ahb_freq, ahb_freq),
            pre => {
                let pre: u8 = pre.into();
                let pre: u8 = 1 << (pre - 3);
                let freq = ahb_freq / pre as u32;
                (freq, freq * 2)
            }
        };

        let (apb2_freq, apb2_tim_freq) = match cfgr.apb2_pre {
            APBPrescaler::NotDivided => (ahb_freq, ahb_freq),
            pre => {
                let pre: u8 = pre.into();
                let pre: u8 = 1 << (pre - 3);
                let freq = ahb_freq / pre as u32;
                (freq, freq * 2)
            }
        };

        let pwr = pac::PWR;
        if cfgr.low_power_run {
            assert!(sys_clk.hz() <= 2_000_000.hz());
            unsafe {
                pwr.cr1().modify(|w| w.set_lpr(true));
            }
        }

        Clocks {
            sys: sys_clk.hz(),
            ahb1: ahb_freq.hz(),
            ahb2: ahb_freq.hz(),
            apb1: apb1_freq.hz(),
            apb1_tim: apb1_tim_freq.hz(),
            apb2: apb2_freq.hz(),
            apb2_tim: apb2_tim_freq.hz(),
        }
    }
}

pub unsafe fn init(config: Config) {
    let r = <peripherals::RCC as embassy::util::Steal>::steal();
    let clocks = r.freeze(config);
    set_freqs(clocks);
}
