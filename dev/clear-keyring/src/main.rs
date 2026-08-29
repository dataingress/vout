const KEYRING_SERVICE: &str = "vout";
const KEYRING_USER: &str = concat!("vout_dbkey_", env!("VOUT_KEYRING_KEY_VERSION"));

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let entry = keyring::Entry::new(KEYRING_SERVICE, KEYRING_USER)?;

    match entry.delete_credential() {
        Ok(()) => {
            println!("deleted keyring entry service={KEYRING_SERVICE} user={KEYRING_USER}");
        }
        Err(keyring::Error::NoEntry) => {
            println!(
                "keyring entry was already absent service={KEYRING_SERVICE} user={KEYRING_USER}"
            );
        }
        Err(err) => return Err(Box::new(err)),
    }

    Ok(())
}
