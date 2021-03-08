/// Trait that allows formatting input to a short/prettier format
pub trait PrettyFormat {
    /// Denotes the min length required for formatting
    const MIN_LENGTH: usize;
    /// Method to convert input to prettier format
    fn pretty(&self) -> Result<Vec<u8>, &'static str>;
}

impl<T> PrettyFormat for T
where
    T: AsRef<[u8]>,
{
    const MIN_LENGTH: usize = 4;

    /// Gives a short friendly transaction-id from a transaction hash
    /// by encoding the first 32bits to base58
    fn pretty(&self) -> Result<Vec<u8>, &'static str> {
        let _x = self.as_ref();
        if _x.len() >= Self::MIN_LENGTH {
            Ok(bs58::encode(&_x[0..Self::MIN_LENGTH]).into_vec())
        } else {
            Err("Input too small!")
        }
    }
}

#[test]
fn test_pretty() {
    assert_eq!(
        "d7bd44af293e45e0dc7583d12".pretty(),
        Ok("3ZaN9y".as_bytes().to_vec())
    );
    assert_eq!("foo".pretty(), Err("Input too small!"));
}
