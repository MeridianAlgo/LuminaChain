pub fn verify_green_energy_proof(proof: &[u8]) -> bool {
    if proof.len() < 32 {
        return false;
    }

    let claimed_tag = &proof[..32];
    let payload = &proof[32..];
    let mut hasher = blake3::Hasher::new();
    hasher.update(b"green-energy");
    hasher.update(payload);

    hasher.finalize().as_bytes() == claimed_tag
}
