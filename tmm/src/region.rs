use core::mem::Alignment;

use alloc::alloc::Allocator;
use cake::{Rng, log::error, rand::RngExt, securand::SecuRng};

use crate::{
    align,
    paging::{Address, VirtAddr, primitives::VirtRange},
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

impl MemoryRegion {
    /// Creates a new `MemoryRegion` with the given params.
    pub fn new(range: VirtRange, name: &'static str) -> Self {
        Self { range, name }
    }
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
pub struct RegionManager<A: Allocator + Clone, const INLINE_CAP: usize = 0x20> {
    managed_range: VirtRange,
    backing: RegionBacking<INLINE_CAP, A>,
}

fn range_can_fulfill(free: VirtRange, req: &RegionRequest) -> Option<VirtRange> {
    if free.size() < req.size {
        return None;
    }

    let range_start = align!(up, free.start().as_u64(), req.alignment.as_usize() as u64);
    let range_end = align!(
        down,
        (free.end() - req.size).as_u64(),
        req.alignment.as_usize() as u64
    );

    // Handle the edge case where the aligned range is outside the free range.
    if range_start > free.end().as_u64() || range_end < free.start().as_u64() {
        return None;
    }

    Some(VirtRange::new(
        VirtAddr::new(range_start),
        VirtAddr::new(range_end),
    ))
}

fn try_fullfil_with_range<R: Rng>(
    r: &mut R,
    free: VirtRange,
    req: &RegionRequest,
) -> Option<MemoryRegion> {
    let range = range_can_fulfill(free, req)?;

    let base = r.random_range(range.start().as_u64()..=range.end().as_u64());

    let aligned_base = align!(down, base, req.alignment.as_usize() as u64);
    let allocated_range = VirtRange::new(
        VirtAddr::new(aligned_base),
        VirtAddr::new(aligned_base + req.size),
    );

    // Sanity check to ensure the allocated range is within the free range.
    debug_assert!(free.contains(allocated_range));

    Some(MemoryRegion::new(allocated_range, req.name))
}

impl<A: Allocator + Clone, const INLINE_CAP: usize> RegionManager<A, INLINE_CAP> {
    /// Creates a new range manager given the range and backing.
    pub const fn new(managed_range: VirtRange, backing: RegionBacking<INLINE_CAP, A>) -> Self {
        Self {
            managed_range,
            backing,
        }
    }

    fn try_fullfil<R: Rng>(
        &self,
        r: &mut R,
        first_idx: usize,
        req: RegionRequest,
    ) -> Option<MemoryRegion> {
        let r0 = &self.backing[first_idx];
        let r1 = &self.backing[first_idx + 1];
        let free_range = VirtRange::new(r0.range.end(), r1.range.start());

        try_fullfil_with_range(r, free_range, &req)
    }

    /// Fulfils the given range request. This does not update the backing.
    pub fn fulfill_raw<R: Rng>(&self, r: &mut R, req: RegionRequest) -> Option<MemoryRegion> {
        if self.backing.len() == 0 {
            return try_fullfil_with_range(r, self.managed_range, &req);
        }

        if self.backing.len() == 1 {
            // In most cases there will be a reserved region where limine loads the kernel, so there's probably no space above it.
            // So, we try and allocate in the lower free range first, and then the upper free range if that fails.
            // TODO: This has security issues when userspace gets implemented, as if we load the binary at a relatively low address,
            // there will probably always be a used range below it. This probably isn't a big deal, but might be worth investigating later.
            let lower_free_range =
                VirtRange::new(self.managed_range.start(), self.backing[0].range.start());
            if let Some(region) = try_fullfil_with_range(r, lower_free_range, &req) {
                return Some(region);
            }

            let upper_free_range =
                VirtRange::new(self.backing[0].range.end(), self.managed_range.end());
            if let Some(region) = try_fullfil_with_range(r, upper_free_range, &req) {
                return Some(region);
            }
        }

        const MAX_ATTEMPTS: usize = 1000;
        for _ in 0..MAX_ATTEMPTS {
            let idx = r.random_range(0..self.backing.len() - 1);
            if let Some(region) = self.try_fullfil(r, idx, req) {
                return Some(region);
            }
        }

        error!(
            "Failed to fulfill region request after {} attempts. This is likely a bug, or, you're just really unlucky.",
            MAX_ATTEMPTS
        );

        return None;
    }

    /// Commits the given memory region to the backing, ensuring that it does not overlap with any existing regions.
    pub fn commit(&mut self, region: MemoryRegion) {
        self.backing
            .binary_search_by_key(&region.range.start(), |r| r.range.start())
            .map_or_else(
                |_| {
                    panic!(
                        "Attempted to commit a region that overlaps with an existing region: {:?}",
                        region
                    )
                },
                |idx| self.backing.insert(idx, region),
            );
    }

    /// Fulfils the given range request and commits it to the backing.
    pub fn fulfill_and_commit<R: Rng>(
        &mut self,
        r: &mut R,
        req: RegionRequest,
    ) -> Option<MemoryRegion> {
        let region = self.fulfill_raw(r, req)?;
        self.commit(region);
        Some(region)
    }
}
