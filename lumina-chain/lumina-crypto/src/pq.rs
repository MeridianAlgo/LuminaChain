use anyhow::{anyhow, bail, Result};

#[cfg(feature = "pq-crypto")]
use pqcrypto_dilithium::dilithium3;
#[cfg(feature = "pq-crypto")]
use pqcrypto_kyber::kyber768;
#[cfg(feature = "pq-crypto")]
use pqcrypto_traits::kem::{Ciphertext as _, PublicKey as _, SecretKey as _, SharedSecret as _};
#[cfg(feature = "pq-crypto")]
use pqcrypto_traits::sign::{DetachedSignature as _, PublicKey as _};

/// Verify Dilithium detached signatures when `pq-crypto` is enabled.
pub fn verify_dilithium_signature(
    public_key: &[u8],
    message: &[u8],
    signature: &[u8],
) -> Result<()> {
    #[cfg(feature = "pq-crypto")]
    {
        let pk = dilithium3::PublicKey::from_bytes(public_key)
            .map_err(|_| anyhow!("Invalid Dilithium public key bytes"))?;
        let sig = dilithium3::DetachedSignature::from_bytes(signature)
            .map_err(|_| anyhow!("Invalid Dilithium signature bytes"))?;
        dilithium3::verify_detached_signature(&sig, message, &pk)
            .map_err(|_| anyhow!("Dilithium signature verification failed"))
    }

    #[cfg(not(feature = "pq-crypto"))]
    {
        let _ = (public_key, message, signature);
        bail!(
            "Post-quantum verification disabled at compile-time. Rebuild with --features pq-crypto"
        )
    }
}

/// Kyber768 KEM decapsulation hook for wallet/custodian MPC channels.
pub fn kyber_decapsulate(secret_key: &[u8], ciphertext: &[u8]) -> Result<Vec<u8>> {
    #[cfg(feature = "pq-crypto")]
    {
        let sk = kyber768::SecretKey::from_bytes(secret_key)
            .map_err(|_| anyhow!("Invalid Kyber secret key bytes"))?;
        let ct = kyber768::Ciphertext::from_bytes(ciphertext)
            .map_err(|_| anyhow!("Invalid Kyber ciphertext bytes"))?;
        let ss = kyber768::decapsulate(&ct, &sk);
        Ok(ss.as_bytes().to_vec())
    }

    #[cfg(not(feature = "pq-crypto"))]
    {
        let _ = (secret_key, ciphertext);
        bail!("Kyber hook disabled at compile-time. Rebuild with --features pq-crypto")
    }
}

/// Kyber768 encapsulation hook for native wallets (no external KMS).
pub fn kyber_encapsulate(public_key: &[u8]) -> Result<(Vec<u8>, Vec<u8>)> {
    #[cfg(feature = "pq-crypto")]
    {
        let pk = kyber768::PublicKey::from_bytes(public_key)
            .map_err(|_| anyhow!("Invalid Kyber public key bytes"))?;
        let (ss, ct) = kyber768::encapsulate(&pk);
        Ok((ct.as_bytes().to_vec(), ss.as_bytes().to_vec()))
    }

    #[cfg(not(feature = "pq-crypto"))]
    {
        let _ = public_key;
        bail!("Kyber hook disabled at compile-time. Rebuild with --features pq-crypto")
    }
}
