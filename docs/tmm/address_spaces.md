# TMM Address Spaces

Tau takes a slightly different approach to address spaces than most operating systems. In Tau, each core has its own address space object, which is managed by the TMM. Since each core has its own address space, most memory management operations are core-local, which avoids a huge amount of locking. Locking is still required when allocating physical memory, but this can be remedied in the future.

Each address space has two [RegionManagers](region.md), one for the higher half (kernel space) and one for the lower half (user space). They also contain a local VirtualMemoryManager (VMM) for fast virtual memory allocations. Each core will have it's own heap. 

There is a global `RegionManager` which is used as a basis for all subsequent address spaces. This is used to ensure that when sharing memory is explicitly requested, the memory is shared between all cores.

Now we have the tools to communicate between cores, but how do we actually share memory? How do we create an address space?

## Default regions

These are the regions that are created by default. Position is not provided as the RM can place them anywhere in the address space. The default regions are:

`SHM`: The shared memory region. This is used for sharing memory between cores. This region is GIANT, taking up a whole pml4 entry. This is because we ..(side note: can you atomically write to a page table entry? opens a lot of doors..) share the entry with all cores. This means that there is a lock, however, that's explicitly why this is opt-in. 
`VMM`: The virtual memory manager region. This is used for allocating virtual memory. This region is allocated to be large, but it follows heap rules in terms of growing.
`KERNEL`: Upper 2 GiB of the address space. This is where the kernel is mapped. This region is not growable, but may be places elsewhere if limine does KASLR. This is also shared between all cores, though probably at a lower granularity than the SHM region. 

## Sharing memory between cores

Sharing memory between cores is done by