//! C standard-library allocation lifecycle strategies.

use core::ptr::NonNull;

use ffibox::CDropped;

use crate::ffi;

/// Wraps: free
/// Deleter strategy for allocations made by the C `malloc` family.
pub struct MallocFree;

// SAFETY: `c_drop` delegates to C `free`, which releases exactly one compatible
// allocation. `CVoidBox::from_raw` requires callers to provide such an allocation.
unsafe impl CDropped for MallocFree {
    unsafe fn c_drop(obj: NonNull<Self>) {
        // SAFETY: the `CDropped` contract guarantees that `obj` is a uniquely
        // owned allocation compatible with the C `malloc` family.
        unsafe { ffi::free(obj.as_ptr().cast()) }
    }
}

#[cfg(test)]
mod tests {
    use ffibox::CVoidBox;

    use super::*;

    #[test]
    fn malloc_allocation_is_released_on_drop() {
        // SAFETY: allocating an opaque byte block has no additional caller
        // obligations; the returned pointer is checked below.
        let raw = unsafe { ffi::malloc(32) };
        // SAFETY: `raw` is null or a fresh, uniquely owned allocation from the
        // C `malloc` family, whose matching strategy is `MallocFree`.
        let owned = unsafe { CVoidBox::<MallocFree>::from_raw(raw) }
            .expect("C malloc should allocate the test block");

        drop(owned);
    }
}
