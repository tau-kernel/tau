# Physical and Virtual Memory Management in Tau

Tau contains two memory managers: the VirtualMemoryManager (VMM) and the PhysicalMemoryManager (PMM). The VMM is responsible for managing virtual memory, while the PMM is responsible for managing physical memory. Each one is based on a bitmap, which is very efficient and has plenty of room for even further optimizations. 


## The outside API 

The actual API may change in the future, but they should be roughly as follows:

```rust
fn reserve_virt(layout: Layout, flags: VirtFlags) -> Result<VirtAddr, MemError>;
fn release_virt(addr: VirtAddr, layout: Layout, flags: VirtFlags) -> Result<(), MemError>;

fn reserve_frame<S: FragmentSize>(frame: Frame<S>) -> Result<(), MemError>;
fn release_frame<S: FragmentSize>(frame: Frame<S>) -> Result<(), MemError>;
```

Notice that physical memory is allocated in terms of frames. This is a restriction that I would like to keep, but may be necessary for like PCI BARs down the line