use chacha20poly1305::{ChaCha20Poly1305, KeyInit, Nonce, aead::Aead};

const FORMAT_VERSION: u8 = 1;
const NONCE_LEN: usize = 12;

pub fn encrypt_with_key(key: &[u8; 32], value: &[u8], aad: &[u8]) -> anyhow::Result<Vec<u8>> {
    let cipher = ChaCha20Poly1305::new_from_slice(key)?;
    let nonce_bytes = crate::crypto::rand::random_array::<NONCE_LEN>();
    let nonce = Nonce::try_from(nonce_bytes.as_slice())?;
    let cipher_text = cipher
        .encrypt(&nonce, chacha20poly1305::aead::Payload { msg: value, aad })
        .map_err(|e| anyhow::anyhow!("parameter encrypt: {e}"))?;

    let mut result = Vec::with_capacity(1 + NONCE_LEN + cipher_text.len());
    result.push(FORMAT_VERSION);
    result.extend_from_slice(&nonce_bytes);
    result.extend_from_slice(&cipher_text);

    Ok(result)
}

pub fn encrypt(value: &[u8], aad: &[u8]) -> anyhow::Result<Vec<u8>> {
    encrypt_with_key(crate::app::settings::get().db_key, value, aad)
}

pub fn decrypt_with_key(key: &[u8; 32], value: &[u8], aad: &[u8]) -> anyhow::Result<Vec<u8>> {
    anyhow::ensure!(
        value.len() > 1 + NONCE_LEN,
        "parameter ciphertext too short"
    );
    anyhow::ensure!(
        value[0] == FORMAT_VERSION,
        "unsupported parameter ciphertext version"
    );

    let cipher = ChaCha20Poly1305::new_from_slice(key)?;
    let nonce = Nonce::try_from(&value[1..1 + NONCE_LEN])?;

    cipher
        .decrypt(
            &nonce,
            chacha20poly1305::aead::Payload {
                msg: &value[1 + NONCE_LEN..],
                aad,
            },
        )
        .map_err(|e| anyhow::anyhow!("parameter decrypt: {e}"))
}

pub fn decrypt(value: &[u8], aad: &[u8]) -> anyhow::Result<Vec<u8>> {
    decrypt_with_key(crate::app::settings::get().db_key, value, aad)
}
