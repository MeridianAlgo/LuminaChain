pub use ed25519_dalek::{SigningKey, VerifyingKey};
use ed25519_dalek::{Signature, Signer, Verifier};
use rand::rngs::OsRng;
use anyhow::{Result, bail};

pub fn generate_keypair() -> SigningKey {
    let mut csprng = OsRng;
    SigningKey::generate(&mut csprng)
}

pub fn sign(key: &SigningKey, message: &[u8]) -> Vec<u8> {
    let sig: Signature = key.sign(message);
    sig.to_bytes().to_vec()
}

pub fn verify_signature(pubkey_bytes: &[u8; 32], message: &[u8], signature_bytes: &[u8]) -> Result<()> {
    let pubkey = VerifyingKey::from_bytes(pubkey_bytes).map_err(|_| anyhow::anyhow!("Invalid public key"))?;
    
    if signature_bytes.len() != 64 {
        bail!("Invalid signature length");
    }
    
    let signature = Signature::from_bytes(signature_bytes.try_into().unwrap());
    
    pubkey.verify(message, &signature).map_err(|_| anyhow::anyhow!("Signature verification failed"))?;
    Ok(())
}
