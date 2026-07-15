#![cfg_attr(not(feature = "std"), no_std)]

extern crate bchlib_sys as ffi;
unsafe impl Send for BCH {}

#[derive(Debug)]
pub struct BCH(*mut ffi::bch_control);

impl BCH {
    pub fn init(m: i32, t: i32) -> Result<BCH, &'static str> {
        BCH::init_with_poly(m, t, 0)
    }

    pub fn check_free() -> i32 {
        unsafe { ffi::bch_check_free() }
    }

    pub fn init_with_poly(m: i32, t: i32, poly: u32) -> Result<BCH, &'static str> {
        unsafe {
            let bch = ffi::init_bch(m, t, poly);
            if bch.is_null() {
                Err("Invalid BCH params")
            } else {
                Ok(BCH(bch))
            }
        }
    }

    #[cfg(feature = "decode")]
    pub fn decode_bits(&mut self, msg: &[u8], ecc: &[u8], errloc: &mut [u32]) -> i32 {
        unsafe { ffi::decodebits_bch(self.0, msg.as_ptr(), ecc.as_ptr(), errloc.as_mut_ptr()) }
    }

    pub fn encode_bits(&mut self, msg: &[u8], ecc: &mut [u8]) {
        unsafe {
            ffi::encodebits_bch(self.0, msg.as_ptr(), ecc.as_mut_ptr());
        };
    }

    #[cfg(feature = "decode")]
    pub fn decode(&mut self, msg: &[u8], ecc: &[u8], errloc: &mut [u32]) -> i32 {
        unsafe {
            ffi::decode_bch(
                self.0,
                msg.as_ptr(),
                msg.len() as u32,
                ecc.as_ptr(),
                core::ptr::null(),
                core::ptr::null(),
                errloc.as_mut_ptr(),
            )
        }
    }

    pub fn encode(&mut self, msg: &[u8], ecc: &mut [u8]) {
        unsafe {
            ffi::encode_bch(self.0, msg.as_ptr(), msg.len() as u32, ecc.as_mut_ptr());
        };
    }

    #[cfg(feature = "decode")]
    pub fn correct(&mut self, msg: &mut [u8], errloc: &[u32], nerr: i32) {
        if nerr <= 0 {
            return;
        }
        unsafe {
            ffi::correct_bch(
                self.0,
                msg.as_mut_ptr(),
                msg.len() as u32,
                errloc.as_ptr() as *mut u32,
                nerr,
            );
        };
    }
}

impl Drop for BCH {
    fn drop(&mut self) {
        // The static-heap allocator (used whenever `malloc` is off) doesn't
        // support freeing individual instances: free_bch's non-malloc path
        // just resets the whole shared bump allocator, which is only sound
        // if every live instance is dropped in strict reverse-creation
        // order - nothing enforces that here, and getting it wrong would
        // silently corrupt a still-alive instance rather than crash. So
        // only actually free when a real allocator backs bch_alloc;
        // otherwise this is a no-op, matching the "one persistent instance
        // for the process lifetime" design intent of the static heap on
        // embedded/no_std targets.
        #[cfg(feature = "malloc")]
        unsafe {
            ffi::free_bch(self.0);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[cfg(feature = "decode")]
    fn test_decode() {
        let mut bch = BCH::init(5, 2).unwrap();
        let msg: [u8; 21] = [
            1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        ];
        let ecc: [u8; 10] = [1, 1, 1, 0, 1, 1, 0, 1, 0, 0];
        let mut errloc: [u32; 2] = [0, 0];
        bch.decode_bits(&msg, &ecc, &mut errloc);
        assert_eq!(errloc[0], 0);
        assert_eq!(errloc[1], 0);
    }

    #[test]
    #[cfg(feature = "decode")]
    fn test_decode_err() {
        let mut bch = BCH::init(5, 2).unwrap();
        let msg: [u8; 21] = [
            1, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        ];
        let ecc: [u8; 10] = [1, 1, 1, 0, 1, 1, 0, 1, 0, 0];
        let mut errloc: [u32; 2] = [0, 0];
        bch.decode_bits(&msg, &ecc, &mut errloc);
        assert_eq!(errloc[0], 9);
        assert_eq!(errloc[1], 0);
    }

    #[test]
    #[cfg(feature = "decode")]
    fn test_sync_codeword() {
        let mut bch = BCH::init(5, 2).unwrap();
        let msg: [u8; 21] = [
            0, 1, 1, 1, 1, 1, 0, 0, 1, 1, 0, 1, 0, 0, 1, 0, 0, 0, 0, 1, 0,
        ];
        let ecc: [u8; 10] = [1, 0, 1, 1, 1, 0, 1, 1, 0, 0];
        let mut errloc: [u32; 2] = [0, 0];
        bch.decode_bits(&msg, &ecc, &mut errloc);
        assert_eq!(errloc[0], 0);
        assert_eq!(errloc[1], 0);
    }

    #[test]
    fn test_init_fail() {
        let bch = BCH::init_with_poly(5, 2, 1897);
        assert_eq!(bch.is_err(), true);
    }

    // Regression test: free_bch's top-level branch used to key off
    // `#ifdef __linux__` (the real host platform) instead of `#ifdef
    // BCH_USE_MALLOC` (the Cargo feature that actually determines whether
    // bch_alloc uses the static heap). On real Linux that meant a failed
    // init_bch() call (which calls free_bch internally on its `fail:` path)
    // took the "call bch_unalloc on each field" branch - a no-op without
    // `malloc` - instead of resetting alloc_heap_i, so the static heap
    // silently accumulated across every failed init call and eventually
    // ran out mid test-suite. This was masked on macOS, where __linux__ is
    // undefined and the reset happened to fire anyway - which is exactly
    // why it only broke CI, not local runs. This checks the actual
    // observable contract directly (a failed init must give back whatever
    // it consumed), so it fails on any platform if that regresses, not
    // just Linux.
    #[test]
    #[cfg(not(feature = "malloc"))]
    fn repeated_failed_init_does_not_leak_static_heap() {
        let free_before = BCH::check_free();
        for _ in 0..5 {
            let bch = BCH::init_with_poly(5, 2, 1897); // invalid primitive polynomial
            assert!(bch.is_err(), "expected invalid poly to fail init");
        }
        let free_after = BCH::check_free();
        assert_eq!(
            free_before, free_after,
            "a failed init_bch() call leaked static heap space instead of resetting it"
        );
    }

    // Regression test: on the static-heap allocator (no `malloc` feature),
    // Drop is a no-op (see impl Drop for BCH), so init_bch() never resets
    // or frees between independent calls - a second BCH::init() starts
    // allocating from wherever the first call's allocations left off.
    // bch_alloc used to hand out pointers without rounding up to an
    // alignment boundary, so this second instance's `struct bch_control`
    // could come back misaligned and crash as soon as it was read
    // (`Ok(BCH(*bch))`, back when init_with_poly copied the struct by value
    // instead of storing the pointer). This reproduces that call pattern
    // and checks the second instance actually works, not just that it
    // avoids crashing.
    #[test]
    #[cfg(feature = "decode")]
    fn test_repeated_init_second_instance_still_works() {
        let _first = BCH::init(5, 2).unwrap();
        let mut second = BCH::init(5, 2).unwrap();

        let msg: [u8; 21] = [
            1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        ];
        let ecc: [u8; 10] = [1, 1, 1, 0, 1, 1, 0, 1, 0, 0];
        let mut errloc: [u32; 2] = [0, 0];
        second.decode_bits(&msg, &ecc, &mut errloc);
        assert_eq!(errloc[0], 0);
        assert_eq!(errloc[1], 0);
    }

    // Exhaustive sweep of the entire valid (m, t) parameter space: for every
    // m in 5..=15, every t from 1 up to init_bch's own limit (the largest t
    // with m*t < (1 << m) - 1, per its `(t < 1) || (m*t >= ((1 << m)-1))`
    // sanity check). For each pair, encodes an all-zero codeword, confirms a
    // clean decode reports zero errors, then flips a single data bit at the
    // start, middle, and end and confirms decode locates it exactly.
    //
    // Requires `malloc`: several (m, t) combinations near the top of the
    // range (e.g. m=15, t=2184) need multi-megabyte tables that don't fit
    // the static heap's fixed 24576-byte pool - that's an expected capacity
    // limit of the embedded allocator, not a correctness bug, so this test
    // is only meaningful with the real allocator backing it.
    //
    // This is `#[ignore]`d by default: it's ~4700 (m, t) pairs, several of
    // which build multi-megabyte tables, so it's slow relative to the rest
    // of the suite. Run it explicitly with:
    //   cargo test --release -- --ignored exhaustive_m_t_sweep
    #[test]
    #[ignore]
    #[cfg(all(feature = "decode", feature = "malloc", feature = "std"))]
    fn exhaustive_m_t_sweep() {
        for m in 5..=15i32 {
            let n = (1u32 << m) - 1;
            // largest t such that m*t < n, matching init_bch's own check
            let t_max = ((n - 1) / m as u32) as i32;

            for t in 1..=t_max {
                let mut bch = BCH::init(m, t)
                    .unwrap_or_else(|e| panic!("init_bch({}, {}) failed: {}", m, t, e));

                let (n, ecc_bits_raw) = unsafe { ((*bch.0).n, (*bch.0).ecc_bits) };
                let k = (n - ecc_bits_raw) as usize;
                let ecc_bits = ecc_bits_raw as usize;

                let msg = vec![0u8; k];
                let mut ecc = vec![0u8; ecc_bits];
                bch.encode_bits(&msg, &mut ecc);

                let mut errloc = vec![0u32; t as usize];
                let nerr = bch.decode_bits(&msg, &ecc, &mut errloc);
                assert_eq!(nerr, 0, "m={} t={}: false positive on clean codeword", m, t);

                for &flip in &[0usize, k / 2, k - 1] {
                    let mut corrupted = msg.clone();
                    corrupted[flip] ^= 1;

                    let mut errloc = vec![0u32; t as usize];
                    let nerr = bch.decode_bits(&corrupted, &ecc, &mut errloc);
                    assert_eq!(
                        nerr, 1,
                        "m={} t={} flip={}: expected exactly 1 error, got {}",
                        m, t, flip, nerr
                    );
                    assert_eq!(
                        errloc[0] as usize, flip,
                        "m={} t={} flip={}: decode located the error at {} instead",
                        m, t, flip, errloc[0]
                    );
                }
            }
        }
    }
}
