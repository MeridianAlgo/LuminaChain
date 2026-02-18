use anyhow::{anyhow, bail, Result};
use ark_bls12_381::{Bls12_381, G1Affine, G1Projective, G2Affine};
use ark_ec::{pairing::Pairing, AffineRepr, CurveGroup};
use ark_serialize::{CanonicalDeserialize, CanonicalSerialize};

/// Aggregate multiple BLS signatures (G1 points) into one signature.
pub fn aggregate_signatures(signature_bytes: &[Vec<u8>]) -> Result<Vec<u8>> {
    if signature_bytes.is_empty() {
        bail!("At least one signature is required");
    }

    let mut agg = G1Projective::default();
    for bytes in signature_bytes {
        let sig = G1Affine::deserialize_compressed(bytes.as_slice())
            .map_err(|_| anyhow!("Invalid BLS signature bytes"))?;
        if sig.is_zero() {
            bail!("Zero signature is not allowed");
        }
        agg += sig;
    }

    let mut out = Vec::new();
    agg.into_affine()
        .serialize_compressed(&mut out)
        .map_err(|_| anyhow!("Failed to serialize aggregated signature"))?;
    Ok(out)
}

/// Verify an aggregated BLS signature over a common message hash point in G2.
///
/// `message_hash_g2` must be a domain-separated hash-to-curve output generated off-chain
/// with an agreed ciphersuite and DST.
pub fn verify_aggregated_signature_same_message(
    pubkeys_g1: &[Vec<u8>],
    aggregated_sig_g1: &[u8],
    message_hash_g2: &[u8],
) -> Result<()> {
    if pubkeys_g1.is_empty() {
        bail!("At least one public key is required");
    }

    let sig = G1Affine::deserialize_compressed(aggregated_sig_g1)
        .map_err(|_| anyhow!("Invalid aggregated signature bytes"))?;
    let msg = G2Affine::deserialize_compressed(message_hash_g2)
        .map_err(|_| anyhow!("Invalid message hash point bytes"))?;

    let mut agg_pk = G1Projective::default();
    for pk_bytes in pubkeys_g1 {
        let pk = G1Affine::deserialize_compressed(pk_bytes.as_slice())
            .map_err(|_| anyhow!("Invalid public key bytes"))?;
        if pk.is_zero() {
            bail!("Zero public key is not allowed");
        }
        agg_pk += pk;
    }

    let lhs = Bls12_381::pairing(sig, G2Affine::generator());
    let rhs = Bls12_381::pairing(agg_pk.into_affine(), msg);

    if lhs == rhs {
        Ok(())
    } else {
        bail!("BLS aggregate verification failed")
    }
}
