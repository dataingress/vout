use rand::{RngCore, SeedableRng};
use rand_chacha::ChaCha8Rng;

pub fn random_array<const LEN: usize>() -> [u8; LEN] {
    let mut bytes = [0u8; LEN];

    bytes.copy_from_slice(&random_bytes::<u8>(LEN));

    bytes
}

pub fn random_bytes<T>(len: usize) -> Vec<T>
where
    T: From<u8>,
{
    let mut rng = ChaCha8Rng::from_entropy();

    (0..len)
        .map(|_| T::from((rng.next_u32() % 256) as u8))
        .collect()
}
