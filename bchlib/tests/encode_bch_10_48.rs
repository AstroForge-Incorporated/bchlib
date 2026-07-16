use bchlib::BCH;

// Encode test at BCH(10, 48) - the target codec for the memory-constrained
// embedded/no_std deployment discussed throughout this work (see the
// alloc_heap sizing discussion). Uses a 60-byte message to match
// ecc_bytes (also 60 for this (m, t) pair) - a literal 60-in/60-out split
// of the transmitted codeword, which is a more concrete demonstration than
// chasing the BCH code's exact theoretical information rate k/n (578/1023
// = 0.565 here, not quite 1/2, since ecc_bytes is sized from m*t rounded
// up to a byte boundary, not from the actual data/ecc bit split).
//
// This lives in its own integration test file, not the unit test module in
// src/lib.rs, because cargo compiles each file under tests/ into its own
// separate process: bch_alloc's static heap is a C-level static shared by
// every BCH::init() call within one process, and Drop is intentionally a
// no-op for it (dropping doesn't reclaim space - see impl Drop for BCH),
// so every other unit test's init_bch() call permanently eats into the
// same shared budget. Isolating this test in its own process means it
// only needs to fit its own ~74856 bytes (m=10, t=48 with decode compiled
// in), not the cumulative footprint of every other test that happens to
// share a process with it.
//
// Uses the byte-oriented encode() API (not the bit-per-byte one used
// elsewhere in the crate) since at this size the arrays are far more
// manageable: 60 bytes of data and 60 bytes of ecc (n=1023, ecc_bits=445,
// ecc_bytes=60), versus 578+445 individual bit elements with the
// bit-level API.
//
// The pre-determined ecc array below was computed once with encode()
// under a decode-enabled build, and separately confirmed there to decode
// back to `msg` with zero reported errors (decode()'s return value was
// `0`) - matching it here is how an encode-only build confirms its output
// is still decodable, without decode being available to check it directly
// in that configuration.
#[test]
fn test_encode_bch_10_48() {
    let mut bch = BCH::init(10, 48).unwrap();
    let mut msg = [0u8; 60];
    for chunk in msg.chunks_exact_mut(10) {
        chunk.copy_from_slice(b"ASTROFORGE");
    }
    let mut ecc = [0u8; 60];
    bch.encode(&msg, &mut ecc);
    assert_eq!(
        ecc,
        [
            64, 62, 178, 135, 70, 103, 103, 171, 116, 130, 24, 77, 121, 61, 70, 3, 0, 21, 23, 184,
            155, 185, 177, 66, 181, 8, 83, 82, 62, 245, 69, 129, 167, 97, 150, 126, 8, 158, 200,
            70, 223, 237, 72, 140, 170, 215, 200, 116, 41, 14, 200, 93, 187, 71, 183, 200, 0, 0, 0,
            0
        ]
    );
}
