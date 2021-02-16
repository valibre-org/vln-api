/// Gives a short friendly transaction-id from a transaction hash
/// by encoding the first 32bits to base58
/// ```
/// use blockchain::util_txid::get_short_tx_id;
/// let result = get_short_tx_id("0xd7bd44af293e45e0dc7583d12c6e75492410b3d74b01fe87371dc2eea1637a57");
/// assert_eq!("6WquqG".as_bytes().to_vec(), result);
/// ```
pub fn get_short_tx_id(input: &str) -> Vec<u8> {
    let input = if input.starts_with("0x") {
        hex::decode(&input[2..]).unwrap_or_else(|_| input.into())
    } else {
        input.into()
    };
    bs58::encode(&input[0..4]).into_vec()
}
