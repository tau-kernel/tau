use core::{
    mem::Alignment,
    ops::{Deref, DerefMut},
};

use alloc::vec::Vec;
use arrayvec::ArrayVec;

use crate::{
    MapFlags,
    paging::{heapless::HeaplessAllocator, primitives::VirtRange},
    region::backing::RegionBacking,
};

mod backing;

/// Represents a region of memory in the system.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MemoryRegion {
    /// The virtual address range of the memory region.
    pub range: VirtRange,
    /// The name of the memory region.
    pub name: &'static str,
}

/// Represents a request for a memory region, including its name, size, and alignment
#[derive(Debug, Clone, Copy)]
pub struct RegionRequest {
    /// The name of the requested memory region.
    pub name: &'static str,
    /// The size of the requested memory region.
    pub size: u64,
    /// The alignment of the requested memory region.
    pub alignment: Alignment,
}

/// A manager for memory regions in the system.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegionManager {
    managed_range: VirtRange,
    backing: RegionBacking,
}
