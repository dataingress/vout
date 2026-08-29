#!/usr/bin/env bash
set -euo pipefail

service="vout"
user="database-key-ecc-private-v1"
workdir="$(mktemp -d)"

cleanup() {
  rm -rf "$workdir"
}

trap cleanup EXIT

mkdir -p "$workdir/src"

cat > "$workdir/Cargo.toml" <<'TOML'
[package]
name = "clear-vout-db-keyring"
version = "0.0.0"
edition = "2024"

[dependencies]
keyring = { version = "3.6.3", features = ["crypto-rust", "linux-native-sync-persistent"] }
TOML

cat > "$workdir/src/main.rs" <<'RS'
const KEYRING_SERVICE: &str = "vout";
const KEYRING_USER: &str = "vout_dbkey_1";

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
RS

echo "clearing keyring entry service=$service user=$user"
cargo run --quiet --manifest-path "$workdir/Cargo.toml"
