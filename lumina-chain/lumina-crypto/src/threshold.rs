use anyhow::{anyhow, bail, Result};
use rand::thread_rng;
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
        let mut rng = thread_rng();
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
        let sig = SignatureShare::from_bytes(sig_share)
            .map_err(|_| anyhow!("Invalid threshold signature share bytes"))?;
        self.pk_set
            .public_key_share(idx)
            .verify(&sig, message)
            .map_err(|_| anyhow!("Threshold signature share verification failed"))
    }

    pub fn combine_signatures(&self, shares: &[(usize, Vec<u8>)]) -> Result<Vec<u8>> {
        let mut parsed = Vec::with_capacity(shares.len());
        for (idx, bytes) in shares {
            let sig_share = SignatureShare::from_bytes(bytes)
                .map_err(|_| anyhow!("Invalid share at index {idx}"))?;
            parsed.push((*idx, sig_share));
        }

        let combined = self
            .pk_set
            .combine_signatures(parsed.into_iter())
            .map_err(|_| anyhow!("Failed to combine threshold signatures"))?;
        Ok(combined.to_bytes().to_vec())
    }

    pub fn verify_combined(&self, message: &[u8], signature: &[u8]) -> Result<()> {
        let sig = Signature::from_bytes(signature)
            .map_err(|_| anyhow!("Invalid combined threshold signature bytes"))?;
        self.pk_set
            .public_key()
            .verify(&sig, message)
            .map_err(|_| anyhow!("Threshold signature verification failed"))
    }
}

pub fn sign_share(secret_share: &SecretKeyShare, message: &[u8]) -> Vec<u8> {
    secret_share.sign(message).to_bytes().to_vec()
}
