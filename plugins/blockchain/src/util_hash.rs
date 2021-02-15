use blake2::{Blake2b, Digest};
use bs58::encode;
use core::hash::Hasher;
use frame_metadata::StorageHasher;

/// hashes and encodes the provided input with the specified hasher
pub fn hash(hasher: &StorageHasher, input: &str) -> String {
    use StorageHasher::*;
    let input = if input.starts_with("0x") {
        hex::decode(&input[2..]).unwrap_or_else(|_| input.into())
    } else {
        input.into()
    };

    let out = match hasher {
        Blake2_128 => Blake2b::digest(&input).as_slice().to_owned(),
        Blake2_256 => unreachable!(),
        Blake2_128Concat => blake2_concat(&input),
        Twox128 => twox_hash(&input),
        Twox256 => unreachable!(),
        Twox64Concat => twox_hash_concat(&input),
        Identity => input.into(),
    };
    hex::encode(out)
}

fn blake2_concat(input: &[u8]) -> Vec<u8> {
    [Blake2b::digest(input).as_slice(), input].concat()
}

fn twox_hash_concat(input: &[u8]) -> Vec<u8> {
    let mut dest = [0; 8];
    let mut h = twox_hash::XxHash64::with_seed(0);

    h.write(input);
    let r = h.finish();
    use byteorder::{ByteOrder, LittleEndian};
    LittleEndian::write_u64(&mut dest, r);

    [&dest[..], input].concat()
}

fn twox_hash(input: &[u8]) -> Vec<u8> {
    let mut dest: [u8; 16] = [0; 16];

    let mut h0 = twox_hash::XxHash64::with_seed(0);
    let mut h1 = twox_hash::XxHash64::with_seed(1);
    h0.write(input);
    h1.write(input);
    let r0 = h0.finish();
    let r1 = h1.finish();
    use byteorder::{ByteOrder, LittleEndian};
    LittleEndian::write_u64(&mut dest[0..8], r0);
    LittleEndian::write_u64(&mut dest[8..16], r1);

    dest.into()
}

/// Gives a short friendly transaction-id
/// Encode the first 32bits to base58
fn get_short_tx_id(input: &str) -> Vec<u8> {
    let input = if input.starts_with("0x") {
        hex::decode(&input[2..]).unwrap_or_else(|_| input.into())
    } else {
        input.into()
    };
    bs58::encode(&input[0..4]).into_vec()
}

#[test]
fn get_short_tx_id_works() {
    let result =
        get_short_tx_id("0xd7bd44af293e45e0dc7583d12c6e75492410b3d74b01fe87371dc2eea1637a57");
    assert_eq!("6WquqG".as_bytes().to_vec(), result);

    let result_2 = get_short_tx_id("0xed5ad5f258eef6a9745042bde7d46e8a5254c183");
    assert_eq!("74taQq".as_bytes().to_vec(), result_2);
}
