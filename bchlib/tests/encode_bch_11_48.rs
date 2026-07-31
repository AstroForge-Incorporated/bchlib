use bchlib::BCH;

#[test]
fn test_encode_bch_11_48() {
    const MSG_SIZE: usize = 66;
    const ECC_SIZE: usize = MSG_SIZE;
    let mut bch = BCH::init(11, 48).unwrap();
    let mut msg = [0u8; MSG_SIZE];
    const ASTROFORGE: &[u8; 10] = b"ASTROFORGE";
    for chunk in msg.chunks_exact_mut(ASTROFORGE.len()) {
        chunk.copy_from_slice(ASTROFORGE);
    }
    let mut ecc = [0u8; ECC_SIZE];
    bch.encode(&msg, &mut ecc);
    assert_eq!(
        ecc,
        [
            153, 176, 119, 100, 118, 118, 216, 219, 127, 23, 1, 208, 58, 62, 215, 52, 74, 97, 103,
            178, 103, 169, 182, 195, 56, 233, 93, 173, 228, 174, 181, 157, 187, 221, 3, 109, 149,
            190, 120, 156, 210, 160, 253, 110, 80, 50, 15, 157, 199, 137, 214, 185, 191, 231, 98,
            89, 102, 213, 203, 174, 87, 189, 28, 80, 216, 0
        ]
    );
}
