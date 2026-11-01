//! Display output.
use core::convert::Infallible;

use cake::{
    OnceMutex,
    limine::{self},
};
use framebuffer::Framebuffer;

pub mod character;
pub mod color;
mod framebuffer;
pub mod screen_char;
pub mod terminal;

pub use character::get_char;
use tmm::{
    ContextualizedError, MapFlags,
    paging::{Address, MemoryRange, PhysAddr},
};

use crate::{
    bootloader::{self},
    declare_module,
};

/// The global framebuffer instance.
pub static FRAMEBUFFER: OnceMutex<Framebuffer> = OnceMutex::uninitialized();
/// The global terminal instance.
pub static TERMINAL: OnceMutex<terminal::Terminal> = OnceMutex::uninitialized();

declare_module!("display", init);

fn init() -> Result<(), Infallible> {
    FRAMEBUFFER.call_init(|| unsafe { Framebuffer::new(bootloader::framebuffer_info().unwrap()) });
    TERMINAL.call_init(|| terminal::Terminal::new(1, 2));
    Ok(())
}

/// Gets the global terminal instance.
#[macro_export]
macro_rules! terminal {
    () => {
        $crate::display::TERMINAL.get()
    };
}

/// Gets the global framebuffer instance.
#[macro_export]
macro_rules! framebuffer {
    () => {
        $crate::display::FRAMEBUFFER.get()
    };
}

/// Information about the framebuffer.
#[derive(Debug)]
pub struct FramebufferInfo {
    /// The width of the framebuffer.
    pub width: u64,
    /// The height of the framebuffer.
    pub height: u64,
    /// The pitch (e.g. the number of bytes in a row) of the framebuffer.
    pub pitch: u64,
    /// The number of **bits** (*not bytes*) per pixel.
    pub bpp: u16,
    /// A raw pointer to the framebuffer.
    pub mut(self) ptr: *mut u8,
}

impl FramebufferInfo {
    /// Get a raw pointer to the framebuffer.
    pub fn ptr(&self) -> *mut u8 {
        self.ptr
    }
}

unsafe impl Send for FramebufferInfo {}
unsafe impl Sync for FramebufferInfo {}

/// Takes ownership of the framebuffer provided by the bootloader and returns a `FramebufferInfo` struct containing information about it.
pub unsafe fn takeown_framebuffer(
    fb: &limine::framebuffer::Framebuffer<'_>,
) -> Result<FramebufferInfo, ContextualizedError> {
    let old_fb_base = fb.addr();
    let n_bytes = fb.pitch() as usize * fb.height() as usize;
    let old_fb_phys_base =
        PhysAddr::new(old_fb_base as u64 - bootloader::physical_memory_offset().as_u64());

    let new_mapping = tmm::create_phys_mapping(
        MemoryRange::new_len(old_fb_phys_base, n_bytes as u64),
        MapFlags::WRITABLE | MapFlags::CACHE_DISABLE,
    )?;

    Ok(FramebufferInfo {
        width: fb.width(),
        height: fb.height(),
        pitch: fb.pitch(),
        bpp: fb.bpp(),
        ptr: new_mapping.as_mut_ptr(),
    })
}
