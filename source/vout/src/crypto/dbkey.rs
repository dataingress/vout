use base64::Engine;
use chacha20poly1305::{ChaCha20Poly1305, KeyInit, Nonce, aead::Aead};
use hkdf::Hkdf;
use rand::SeedableRng;
use rand_chacha::ChaCha20Rng;
use sha2::Sha256;
use x25519_dalek::{EphemeralSecret, PublicKey, StaticSecret};
use zeroize::Zeroize;

use crate::crypto::rand::random_array;

const AES256_KEY_SIZE: usize = 32;
const X25519_KEY_LEN: usize = 32;

const KEYRING_SERVICE: &str = "vout";
const KEYRING_USER: &str = concat!("vout_dbkey_", env!("VOUT_KEYRING_KEY_VERSION"));
const HKDF_INFO: &[u8] = b"vout-wrap-aes-key";

fn base64_fixed_decode<const LEN: usize>(value: &str) -> anyhow::Result<[u8; LEN]> {
    let result = base64::prelude::BASE64_STANDARD.decode(value)?;

    Ok(result
        .try_into()
        .map_err(|_| anyhow::anyhow!("invalid length"))?)
}

/// creates a AES256 key, 32 bytes
fn create_key() -> [u8; AES256_KEY_SIZE] {
    random_array()
}

fn decode_keyring_key(v: &str) -> anyhow::Result<(StaticSecret, PublicKey)> {
    let private_key = StaticSecret::from(base64_fixed_decode::<X25519_KEY_LEN>(v)?);
    let public_key = PublicKey::from(&private_key);

    Ok((private_key, public_key))
}

fn create_keyring_key() -> anyhow::Result<(StaticSecret, PublicKey)> {
    let mut private_key_bytes = random_array();
    let private_key = StaticSecret::from(private_key_bytes);
    let public_key = PublicKey::from(&private_key);

    private_key_bytes.zeroize();

    Ok((private_key, public_key))
}

fn encrypt_key(
    key: &[u8; AES256_KEY_SIZE],
    recipient_public: &PublicKey,
) -> anyhow::Result<Vec<u8>> {
    let mut rng = ChaCha20Rng::from_entropy();
    let ephemeral = EphemeralSecret::random_from_rng(&mut rng);
    let ephemeral_public = PublicKey::from(&ephemeral);
    let shared = ephemeral.diffie_hellman(recipient_public);
    let mut wrap_key = [0u8; 32];

    if let Err(e) = Hkdf::<Sha256>::new(None, shared.as_bytes()).expand(HKDF_INFO, &mut wrap_key) {
        return Err(anyhow::anyhow!("hkdf expand: {e}"));
    }

    let cipher = ChaCha20Poly1305::new_from_slice(&wrap_key)?;

    wrap_key.zeroize();

    let nonce = Nonce::from([0u8; 12]); // wrap_key is unique per ephemeral
    let cipher_text = cipher
        .encrypt(&nonce, key.as_slice())
        .map_err(|e| anyhow::anyhow!("aead encrypt: {e}"))?;

    let mut result = Vec::with_capacity(32 + 12 + cipher_text.len());

    result.extend_from_slice(ephemeral_public.as_bytes());
    result.extend_from_slice(nonce.as_slice());
    result.extend_from_slice(&cipher_text);

    Ok(result)
}

fn decrypt_key(blob: &[u8], recipient_private: &StaticSecret) -> anyhow::Result<Vec<u8>> {
    anyhow::ensure!(blob.len() > 32 + 12, "ciphertext too short");

    let eph_pub = PublicKey::from(<[u8; 32]>::try_from(&blob[..32])?);
    let nonce = Nonce::try_from(&blob[32..44])?;
    let cipher_text = &blob[44..];

    let shared = recipient_private.diffie_hellman(&eph_pub);
    let mut wrap_key = [0u8; 32];

    if let Err(e) = Hkdf::<Sha256>::new(None, shared.as_bytes()).expand(HKDF_INFO, &mut wrap_key) {
        return Err(anyhow::anyhow!("hkdf expand: {e}"));
    }

    let cipher = ChaCha20Poly1305::new_from_slice(&wrap_key)?;

    wrap_key.zeroize();

    let pt = cipher
        .decrypt(&nonce, cipher_text)
        .map_err(|e| anyhow::anyhow!("aead decrypt: {e}"))?;

    pt.try_into().map_err(|_| anyhow::anyhow!("invalid length"))
}

pub fn create() -> anyhow::Result<String> {
    let entry = keyring::Entry::new(KEYRING_SERVICE, KEYRING_USER)?;

    let (mut private_key, mut public_key) = match entry.get_password() {
        Ok(_) => anyhow::bail!("trying to create key when it exist"),
        Err(keyring::Error::NoEntry) => create_keyring_key()?,
        Err(e) => anyhow::bail!(e),
    };

    let mut private_key_b64 = base64::prelude::BASE64_STANDARD.encode(&private_key.as_bytes());
    let mut key = create_key();
    let encrypted_key = encrypt_key(&key, &public_key)?;

    entry.set_password(&private_key_b64)?;

    key.zeroize();
    private_key_b64.zeroize();
    private_key.zeroize();
    public_key.zeroize();

    Ok(base64::prelude::BASE64_STANDARD.encode(&encrypted_key))
}

pub fn load(encrypted_key: &str) -> anyhow::Result<[u8; AES256_KEY_SIZE]> {
    let entry = keyring::Entry::new(KEYRING_SERVICE, KEYRING_USER)?;

    let (mut private_key, mut public_key) = match entry.get_password() {
        Ok(mut v) => {
            let result = decode_keyring_key(&v)?;
            v.zeroize();
            result
        }
        Err(keyring::Error::NoEntry) => anyhow::bail!("no keyring entry found"),
        Err(e) => anyhow::bail!(e),
    };

    let encrypted_key_bytes = base64::prelude::BASE64_STANDARD.decode(encrypted_key)?;
    let decrypted_key = decrypt_key(&encrypted_key_bytes, &private_key)?;

    private_key.zeroize();
    public_key.zeroize();

    Ok(decrypted_key
        .try_into()
        .map_err(|_| anyhow::anyhow!("invalid length"))?)
}
