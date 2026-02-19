use anyhow::{bail, Result};
use lumina_crypto::signatures::verify_signature;
use lumina_types::state::AccountState;
use std::collections::HashSet;

pub fn recover_social(
    account: &mut AccountState,
    new_device_key: &[u8],
    guardian_signatures: &[Vec<u8>],
) -> Result<()> {
    if account.guardians.is_empty() {
        bail!("Account has no guardians configured");
    }

    let threshold = (account.guardians.len() / 2) + 1;
    if guardian_signatures.len() < threshold {
        bail!(
            "Insufficient guardian signatures: need {}, got {}",
            threshold,
            guardian_signatures.len()
        );
    }

    let mut used_guardians = HashSet::<[u8; 32]>::new();
    let mut verified_count = 0usize;
    for sig in guardian_signatures {
        let mut matched = None;
        for guardian in &account.guardians {
            if used_guardians.contains(guardian) {
                continue;
            }
            if verify_signature(guardian, new_device_key, sig).is_ok() {
                matched = Some(*guardian);
                break;
            }
        }

        if let Some(guardian) = matched {
            used_guardians.insert(guardian);
            verified_count = verified_count.saturating_add(1);
        }
    }

    if verified_count < threshold {
        bail!("Guardian signature verification failed");
    }

    account.passkey_device_key = Some(new_device_key.to_vec());
    Ok(())
}
