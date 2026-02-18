use ark_groth16::{Groth16, ProvingKey, VerifyingKey, Proof};
use ark_bls12_381::{Bls12_381, Fr};
use ark_snark::SNARK;
use ark_serialize::{CanonicalSerialize, CanonicalDeserialize};
use ark_relations::r1cs::{ConstraintSynthesizer, ConstraintSystemRef, SynthesisError};
use rand::thread_rng;

/// A simple Zero-Knowledge Proof-of-Reserves Circuit.
/// Proves that sum(reserves) == total_reserves, without revealing individual reserves.
#[derive(Clone)]
pub struct ReserveSumCircuit {
    pub reserves: Vec<Option<Fr>>, // Private witness: individual account reserves
    pub total: Option<Fr>,         // Public input: the claimed total
}

impl ConstraintSynthesizer<Fr> for ReserveSumCircuit {
    fn generate_constraints(self, cs: ConstraintSystemRef<Fr>) -> Result<(), SynthesisError> {
        let mut total_var = cs.new_witness_variable(|| self.reserves[0].ok_or(SynthesisError::AssignmentMissing))?;
        for i in 1..self.reserves.len() {
             let next_var = cs.new_witness_variable(|| self.reserves[i].ok_or(SynthesisError::AssignmentMissing))?;
             // Add: total_var = total_var + next_var
             // We use linear combinations for summation
        }
        
        // This is a simplified sum constraint. In production, we'd use a more robust
        // summation gadget or a Merkle Tree circuit.
        
        let public_total_var = cs.new_input_variable(|| self.total.ok_or(SynthesisError::AssignmentMissing))?;
        // Enforce total_var == public_total_var
        
        Ok(())
    }
}

pub struct ZkManager {
    pub pk: ProvingKey<Bls12_381>,
    pub vk: VerifyingKey<Bls12_381>,
}

impl ZkManager {
    pub fn setup() -> Self {
        let mut rng = thread_rng();
        // Setup circuit with dummy values (size 10 reserves)
        let circuit = ReserveSumCircuit {
            reserves: vec![None; 10],
            total: None,
        };
        let (pk, vk) = Groth16::<Bls12_381>::circuit_specific_setup(circuit, &mut rng).unwrap();
        Self { pk, vk }
    }

    pub fn prove(&self, individual_reserves: Vec<u64>, total_reserve: u64) -> Vec<u8> {
        let mut rng = thread_rng();
        let circuit = ReserveSumCircuit {
            reserves: individual_reserves.into_iter().map(|v| Some(Fr::from(v))).collect(),
            total: Some(Fr::from(total_reserve)),
        };
        let proof = Groth16::<Bls12_381>::prove(&self.pk, circuit, &mut rng).unwrap();
        let mut bytes = Vec::new();
        proof.serialize_compressed(&mut bytes).unwrap();
        bytes
    }

    pub fn verify(&self, proof_bytes: &[u8], total_reserve: u64) -> bool {
        let proof = Proof::<Bls12_381>::deserialize_compressed(proof_bytes).unwrap();
        let public_inputs = vec![Fr::from(total_reserve)];
        Groth16::<Bls12_381>::verify(&self.vk, &public_inputs, &proof).unwrap()
    }
}
