#![cfg_attr(not(feature = "std"), no_std)]
#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

use core::{concat, env, include};
include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        unsafe {
            let _c = init_bch(5, 2, 37);
        }
    }

    // Regression test for a bug in the static-heap bump allocator
    // (bch_alloc, used when the `malloc` feature is disabled): allocation
    // sizes weren't rounded up to an alignment boundary, so alloc_heap_i
    // could drift to an offset that isn't a multiple of 8. Since nothing
    // ever resets that offset between independent init_bch() calls (there
    // is no free/reset exposed to callers who just want a fresh instance),
    // any init_bch() call after the first could hand back a `struct
    // bch_control*` that isn't aligned for its pointer-sized fields. Reading
    // through such a pointer (e.g. `*bch` in bchlib's BCH::init_with_poly)
    // crashed with "misaligned pointer dereference" on strict-alignment
    // targets (observed on aarch64).
    //
    // Only meaningful without the `malloc` feature: real malloc() is always
    // sufficiently aligned, so this can't reproduce the bug there.
    #[test]
    #[cfg(not(feature = "malloc"))]
    fn repeated_init_bch_returns_aligned_pointers() {
        // Nothing resets the static heap between these calls (there's no
        // free() equivalent exposed for "just give me a fresh instance"),
        // so a handful of iterations is enough to prove pointers stay
        // aligned as the heap offset drifts, without exhausting the fixed
        // 24576-byte pool this (5, 3) configuration allocates several KB
        // from per call. (5, 3) is deliberately chosen over (5, 2): with
        // the `decode` feature off, (5, 2)'s odd-sized allocations happen
        // to cancel out mod 8 after one call, masking the bug; (5, 3)'s
        // don't, so it actually drifts off an 8-byte boundary each call.
        unsafe {
            for i in 0..3 {
                let bch = init_bch(5, 3, 0);
                assert!(!bch.is_null(), "init_bch returned null on iteration {}", i);
                assert_eq!(
                    (bch as usize) % core::mem::align_of::<bch_control>(),
                    0,
                    "init_bch returned a misaligned bch_control pointer on iteration {}",
                    i
                );

                // this is exactly what bchlib::BCH::init_with_poly does;
                // it's what actually crashed before the allocator was fixed.
                let copy = *bch;
                assert_eq!(copy.m, 5);
                assert_eq!(copy.t, 3);
                assert_eq!(copy.n, 31);

                assert_eq!(
                    (copy.a_pow_tab as usize) % core::mem::align_of::<u16>(),
                    0,
                    "a_pow_tab misaligned on iteration {}",
                    i
                );
                assert_eq!(
                    (copy.mod8_tab as usize) % core::mem::align_of::<u32>(),
                    0,
                    "mod8_tab misaligned on iteration {}",
                    i
                );
                assert_eq!(
                    (copy.ecc_buf as usize) % core::mem::align_of::<u32>(),
                    0,
                    "ecc_buf misaligned on iteration {}",
                    i
                );
            }
        }
    }
}
