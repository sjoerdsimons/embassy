#[cfg(bdma)]
mod bdma;
#[cfg(dma)]
mod dma;
#[cfg(dmamux)]
mod dmamux;

#[cfg(dmamux)]
pub use dmamux::*;

use core::future::Future;
use embassy::util::Unborrow;

#[cfg(feature = "unstable-pac")]
pub use transfers::*;

#[cfg(not(feature = "unstable-pac"))]
pub(crate) use transfers::*;

#[cfg(any(bdma_v2, dma_v2, dmamux))]
pub type Request = u8;
#[cfg(not(any(bdma_v2, dma_v2, dmamux)))]
pub type Request = ();

pub(crate) mod sealed {
    use super::*;
    use core::task::Waker;
    pub trait Channel {
        type CompletionFuture<'a>: Future<Output = ()> + 'a
        where
            Self: 'a;

        /// Starts this channel for writing a stream of words.
        unsafe fn start_write<W: Word>(&mut self, request: Request, buf: &[W], reg_addr: *mut u32);

        /// Starts this channel for writing a word repeatedly.
        unsafe fn start_write_repeated<W: Word>(
            &mut self,
            request: Request,
            repeated: W,
            count: usize,
            reg_addr: *mut u32,
        );

        /// Starts this channel for reading a stream of words.
        unsafe fn start_read<W: Word>(
            &mut self,
            request: Request,
            reg_addr: *mut u32,
            buf: &mut [W],
        );

        /// Stops this channel.
        fn stop(&mut self);

        /// Returns whether this channel is active or stopped.
        fn is_stopped(&self) -> bool;

        /// Returns the total number of remaining transfers.
        fn remaining_transfers(&mut self) -> u16;

        /// Sets the waker that is called when this channel completes.
        fn set_waker(&mut self, waker: &Waker);

        fn wait_for_completion<'a>(&mut self) -> Self::CompletionFuture<'a>;
    }
}

pub enum WordSize {
    OneByte,
    TwoBytes,
    FourBytes,
}
pub trait Word {
    fn bits() -> WordSize;
}

impl Word for u8 {
    fn bits() -> WordSize {
        WordSize::OneByte
    }
}

impl Word for u16 {
    fn bits() -> WordSize {
        WordSize::TwoBytes
    }
}
impl Word for u32 {
    fn bits() -> WordSize {
        WordSize::FourBytes
    }
}

mod transfers {
    use super::Channel;
    use embassy_hal_common::{drop::OnDrop, unborrow};

    use super::*;

    #[allow(unused)]
    pub async fn read<'a, W: Word>(
        channel: &mut impl Unborrow<Target = impl Channel>,
        request: Request,
        reg_addr: *mut u32,
        buf: &'a mut [W],
    ) {
        assert!(buf.len() <= 0xFFFF);
        let drop_clone = unsafe { channel.unborrow() };
        unborrow!(channel);

        channel.stop();
        let on_drop = OnDrop::new({
            let mut channel = drop_clone;
            move || {
                channel.stop();
            }
        });

        unsafe { channel.start_read::<W>(request, reg_addr, buf) };
        channel.wait_for_completion().await;
        drop(on_drop)
    }

    #[allow(unused)]
    pub async fn write<'a, W: Word>(
        channel: &mut impl Unborrow<Target = impl Channel>,
        request: Request,
        buf: &'a [W],
        reg_addr: *mut u32,
    ) {
        assert!(buf.len() <= 0xFFFF);
        let drop_clone = unsafe { channel.unborrow() };
        unborrow!(channel);

        channel.stop();
        let on_drop = OnDrop::new({
            let mut channel = drop_clone;
            move || {
                channel.stop();
            }
        });

        unsafe { channel.start_write::<W>(request, buf, reg_addr) };
        channel.wait_for_completion().await;
        drop(on_drop)
    }

    #[allow(unused)]
    pub async fn write_repeated<W: Word>(
        channel: &mut impl Unborrow<Target = impl Channel>,
        request: Request,
        repeated: W,
        count: usize,
        reg_addr: *mut u32,
    ) {
        let drop_clone = unsafe { channel.unborrow() };
        unborrow!(channel);

        channel.stop();
        let on_drop = OnDrop::new({
            let mut channel = drop_clone;
            move || {
                channel.stop();
            }
        });

        unsafe { channel.start_write_repeated::<W>(request, repeated, count, reg_addr) };
        channel.wait_for_completion().await;
        drop(on_drop)
    }
}

pub trait Channel: sealed::Channel + Unborrow<Target = Self> {}

pub struct NoDma;

unsafe impl Unborrow for NoDma {
    type Target = NoDma;

    unsafe fn unborrow(self) -> Self::Target {
        self
    }
}

// safety: must be called only once at startup
pub(crate) unsafe fn init() {
    #[cfg(bdma)]
    bdma::init();
    #[cfg(dma)]
    dma::init();
    #[cfg(dmamux)]
    dmamux::init();
}
