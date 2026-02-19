use anyhow::{bail, Result};

pub fn verify_pq_signature(pq_pubkey: &[u8], message: &[u8], signature: &[u8]) -> Result<()> {
    if pq_pubkey.is_empty() || signature.is_empty() {
        bail!("Empty PQ key or signature");
    }

    crate::pq::verify_dilithium_signature(pq_pubkey, message, signature)
}
