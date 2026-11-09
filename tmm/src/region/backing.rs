use core::ops::{Deref, DerefMut, Index};

use alloc::{alloc::Allocator, vec::Vec};
use arrayvec::ArrayVec;

use crate::{MapFlags, paging::heapless::HeaplessAllocator, region::MemoryRegion};

/// The backing for a region manager.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RegionBacking<const INLINE_CAP: usize = 0x20, A: Allocator = HeaplessAllocator> {
    /// The regions are stored inline. This is mainly used for bootstrapping.
    Inline(ArrayVec<MemoryRegion, INLINE_CAP>, A),
    /// Allocate the regions off-stack.
    Vector(Vec<MemoryRegion, A>),
}

impl<const INLINE_CAP: usize, A: Allocator> RegionBacking<INLINE_CAP, A> {
    /// Creates a new RegionBacking inline.
    pub const fn inline(alloca: A) -> Self {
        Self::Inline(ArrayVec::new_const(), alloca)
    }

    /// Creates a new RegionBacking in a vector.
    pub fn vector(alloca: A) -> Self {
        Self::Vector(Vec::new_in(alloca))
    }

    /// Pushes an element into the backing, moving it to a vec if the capacity is overflowed.
    pub fn push(&mut self, element: MemoryRegion)
    where
        A: Clone,
    {
        match self {
            Self::Inline(av, alloc) => {
                if let Err(_) = av.try_push(element) {
                    *self = Self::Vector(av.to_vec_in(alloc.clone()));
                    self.push(element);
                }
                return;
            }
            Self::Vector(memory_regions) => memory_regions.push(element),
        }
    }
}

impl<const INLINE_CAP: usize, A: Allocator> Deref for RegionBacking<INLINE_CAP, A> {
    type Target = [MemoryRegion];

    fn deref(&self) -> &Self::Target {
        match self {
            RegionBacking::Inline(array_vec, _) => &*array_vec,
            RegionBacking::Vector(memory_regions) => &*memory_regions,
        }
    }
}

impl<const INLINE_CAP: usize, A: Allocator> DerefMut for RegionBacking<INLINE_CAP, A> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        match self {
            RegionBacking::Inline(array_vec, _) => &mut *array_vec,
            RegionBacking::Vector(memory_regions) => &mut *memory_regions,
        }
    }
}
#[cfg(test)]
mod tests {
    use core::assert_matches;
    use std::alloc::Global;

    use arrayvec::ArrayVec;

    use crate::{
        paging::{MemoryRange, VirtAddr},
        region::{MemoryRegion, backing::RegionBacking},
    };

    #[test]
    fn test_backing_push_inline() {
        let mut backing = RegionBacking::Inline(ArrayVec::<_, 1>::new(), Global);
        let r1 = MemoryRegion::new(MemoryRange::new_len(VirtAddr::MIN, 0x5000), "test1");
        backing.push(r1);
        assert_eq!(backing[0], r1);
        assert_matches!(backing, RegionBacking::Inline(_, Global));
        let r2 = MemoryRegion::new(
            MemoryRange::new_len(VirtAddr::MIN + 0x5000, 0x5000),
            "test2",
        );
        backing.push(r2);
        assert_eq!(&*backing, &[r1, r2]);
        assert_matches!(backing, RegionBacking::Vector(_));
    }
}
