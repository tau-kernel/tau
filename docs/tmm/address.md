# `VirtAddr` and `PhysAddr`

`tmm` provides two address types: `VirtAddr` and `PhysAddr`. Both of these types implement the `Address` trait, which provides a common interface for working with addresses. The `Address` trait is defined in the `tmm::address` module.

Both address types are wrappers around a `u64`, and provide methods for converting between the two types, as well as for performing arithmetic on addresses. The `Address` trait provides methods for converting between the two types, as well as for performing arithmetic on addresses.

Addresses are considered the correct way to pass around addresses in not just `tmm`, but also everywhere else in Tau. Rust allows us to guarantee that not only are we passing around the correct type of address, but the intent behind the address is also clear. This is where normal pointers come into play.

## VirtAddr and intent

You might ask, "why not just use a `*mut T` or `*const T` for virtual addresses?" The answer, other than the fact that pointers can be invalid, is intent. A pointer is a pointer. It's easy to assume a pointer is probably safe to dereference, maybe making sure it's not null. There's no distinction between "this is a pointer to a unmapped range of virtual memory" and "this is a pointer to mapped memory". This is where `VirtAddr` comes in. A `VirtAddr` is a virtual address. There's no promise of it being mapped, but it is guaranteed to be a virtual address. This allows us to make sure that we are not accidentally dereferencing a pointer that is not valid, and also allows us to make sure that we are not accidentally passing around a physical address where a virtual address is expected.

`NonNull` pointers should however be used when returning memory that is guaranteed to be mapped, but may be invalid to dereference, either because it's not initialized, or because it's an invalid bit pattern. 

TODO: research covariance because NonNull makes special guarantees and i'm not quite certain how that works. 

This section doesn't really apply to `PhysAddr`, as there's very few cases where dereferencing a physical address is valid, and even then, it's usually only valid in the context of a specific mapping. However, `PhysAddr` is still useful for making sure that we are not accidentally passing around a virtual address where a physical address is expected.