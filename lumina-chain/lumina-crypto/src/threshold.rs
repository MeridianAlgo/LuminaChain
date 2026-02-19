use anyhow::{anyhow, bail, Result};
use rand_core::OsRng;
use threshold_crypto::{PublicKeySet, SecretKeySet, SecretKeyShare, Signature, SignatureShare};

/// DKG-free deterministic threshold config for custodians.
#[derive(Clone)]
pub struct ThresholdConfig {
    pub threshold: usize,
    pub sk_set: SecretKeySet,
    pub pk_set: PublicKeySet,
}

impl ThresholdConfig {
    pub fn new(threshold: usize) -> Self {
        let mut rng = OsRng;
        let sk_set = SecretKeySet::random(threshold, &mut rng);
        let pk_set = sk_set.public_keys();
        Self {
            threshold,
            sk_set,
            pk_set,
        }
    }

    pub fn share_secret_key(&self, idx: usize) -> Result<SecretKeyShare> {
        if idx > u32::MAX as usize {
            bail!("share index out of range");
        }
        Ok(self.sk_set.secret_key_share(idx))
    }

    pub fn verify_share(&self, idx: usize, message: &[u8], sig_share: &[u8]) -> Result<()> {
        if sig_share.len() != 96 {
            bail!(
                "Invalid threshold signature share length: expected 96, got {}",
                sig_share.len()
            );
        }
        let mut sig_arr = [0u8; 96];
        sig_arr.copy_from_slice(sig_share);
        let sig = SignatureShare::from_bytes(sig_arr)
            .map_err(|_| anyhow!("Invalid threshold signature share bytes"))?;

        if !self.pk_set.public_key_share(idx).verify(&sig, message) {
            bail!("Threshold signature share verification failed");
        }
        Ok(())
    }

    pub fn combine_signatures(&self, shares: &[(usize, Vec<u8>)]) -> Result<Vec<u8>> {
        let mut parsed = Vec::with_capacity(shares.len());
        for (idx, bytes) in shares {
            if bytes.len() != 96 {
                bail!(
                    "Invalid share at index {idx}: expected 96 bytes, got {}",
                    bytes.len()
                );
            }
            let mut arr = [0u8; 96];
            arr.copy_from_slice(bytes);
            let sig_share = SignatureShare::from_bytes(arr)
                .map_err(|_| anyhow!("Invalid share at index {idx}"))?;
            parsed.push((*idx, sig_share));
        }

        let combined = self
            .pk_set
            .combine_signatures(parsed.iter().map(|(idx, share)| (*idx, share)))
            .map_err(|_| anyhow!("Failed to combine threshold signatures"))?;
        Ok(combined.to_bytes().to_vec())
    }

    pub fn verify_combined(&self, message: &[u8], signature: &[u8]) -> Result<()> {
        if signature.len() != 96 {
            bail!(
                "Invalid combined threshold signature length: expected 96, got {}",
                signature.len()
            );
        }

        let mut sig_arr = [0u8; 96];
        sig_arr.copy_from_slice(signature);
        let sig = Signature::from_bytes(sig_arr)
            .map_err(|_| anyhow!("Invalid combined threshold signature bytes"))?;

        if !self.pk_set.public_key().verify(&sig, message) {
            bail!("Threshold signature verification failed");
        }
        Ok(())
    }
}

pub fn sign_share(secret_share: &SecretKeyShare, message: &[u8]) -> Vec<u8> {
    secret_share.sign(message).to_bytes().to_vec()
}
