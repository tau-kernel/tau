use core::ops::{Deref, DerefMut};

use alloc::vec::Vec;
use arrayvec::ArrayVec;

use crate::{
    MapFlags,
    paging::heapless::HeaplessAllocator,
    region::MemoryRegion,
};

/// The backing for a region manager.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RegionBacking<const INLINE_CAP: usize = 0x20> {
    /// The regions are stored inline. This is mainly used for bootstrapping.
    Inline(ArrayVec<MemoryRegion, INLINE_CAP>),
    /// Allocate the regions off-stack.
    Vector(Vec<MemoryRegion, HeaplessAllocator>),
}

const REGION_BACKING_ALLOC: HeaplessAllocator = HeaplessAllocator::new(MapFlags::WRITABLE);

impl<const INLINE_CAP: usize> RegionBacking<INLINE_CAP> {
    /// Creates a new RegionBacking inline.
    pub const fn inline() -> Self {
        Self::Inline(ArrayVec::new_const())
    }

    /// Creates a new RegionBacking in a vector.
    pub fn vector() -> Self {
        Self::Vector(Vec::new_in(REGION_BACKING_ALLOC))
    }

    /// Pushes an element into the backing, moving it to a vec if the capacity is overflowed.
    pub fn push(&mut self, element: MemoryRegion) {
        match self {
            Self::Inline(av) => {
                if let Err(_) = av.try_push(element) {
                    if !crate::is_init() {
                        panic!(
                            "RegionBacking::push: ArrayVec Capacity met before memory init! Try increasing INLINE_CAP!"
                        )
                    }

                    *self = Self::Vector(av.to_vec_in(REGION_BACKING_ALLOC));
                    self.push(element);
                }
            }
            Self::Vector(v) => v.push(element),
        }
    }
}

impl<const INLINE_CAP: usize> Deref for RegionBacking<INLINE_CAP> {
    type Target = [MemoryRegion];

    fn deref(&self) -> &Self::Target {
        match self {
            RegionBacking::Inline(array_vec) => &*array_vec,
            RegionBacking::Vector(memory_regions) => &*memory_regions,
        }
    }
}

impl<const INLINE_CAP: usize> DerefMut for RegionBacking<INLINE_CAP> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        match self {
            RegionBacking::Inline(array_vec) => &mut *array_vec,
            RegionBacking::Vector(memory_regions) => &mut *memory_regions,
        }
    }
}

#[cfg(test)]
mod tests {}
