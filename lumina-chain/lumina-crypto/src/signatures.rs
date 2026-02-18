pub use ed25519_dalek::{SigningKey, VerifyingKey};

use anyhow::{bail, Result};
use ed25519_dalek::{Signature, Signer, Verifier};
use rand::rngs::OsRng;

/// Generate a new Ed25519 keypair using OS-level CSPRNG.
pub fn generate_keypair() -> SigningKey {
    let mut csprng = OsRng;
    SigningKey::generate(&mut csprng)
}

/// Sign a message with Ed25519.
pub fn sign(key: &SigningKey, message: &[u8]) -> Vec<u8> {
    let sig: Signature = key.sign(message);
    sig.to_bytes().to_vec()
}

/// Verify an Ed25519 signature against a 32-byte public key.
pub fn verify_signature(
    pubkey_bytes: &[u8; 32],
    message: &[u8],
    signature_bytes: &[u8],
) -> Result<()> {
    let pubkey = VerifyingKey::from_bytes(pubkey_bytes)
        .map_err(|_| anyhow::anyhow!("Invalid public key"))?;

    if signature_bytes.len() != 64 {
        bail!(
            "Invalid signature length: expected 64, got {}",
            signature_bytes.len()
        );
    }

    let mut sig_arr = [0u8; 64];
    sig_arr.copy_from_slice(signature_bytes);
    let signature = Signature::from_bytes(&sig_arr);

    pubkey
        .verify(message, &signature)
        .map_err(|_| anyhow::anyhow!("Signature verification failed"))?;
    Ok(())
}

/// Verify a post-quantum signature using Dilithium hooks.
///
/// This does not simulate cryptography: if `pq-crypto` is not compiled,
/// verification fails closed.
pub fn verify_pq_signature(pq_pubkey: &[u8], message: &[u8], signature: &[u8]) -> Result<()> {
    if pq_pubkey.is_empty() || signature.is_empty() {
        bail!("Empty PQ key or signature");
    }

    crate::pq::verify_dilithium_signature(pq_pubkey, message, signature)
}
