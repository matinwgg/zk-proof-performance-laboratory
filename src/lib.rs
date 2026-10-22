//! Reproducible Groth16 benchmarks over BLS12-381.
//!
//! The circuit is intentionally small and deterministic: one public input is
//! squared repeatedly. Each squaring is one R1CS constraint. This makes the
//! constraint count explicit and lets the harness report how proving and
//! verification scale while keeping the Groth16 proof size constant.

use std::time::Instant;

use ark_bls12_381::{Bls12_381, Fr};
use ark_ff::Field;
use ark_groth16::{prepare_verifying_key, Groth16};
use ark_relations::{
    gr1cs::{ConstraintSynthesizer, ConstraintSystem, ConstraintSystemRef, SynthesisError},
    lc,
};
use ark_serialize::{CanonicalSerialize, Compress};
use ark_snark::{CircuitSpecificSetupSNARK, SNARK};
use ark_std::rand::{rngs::StdRng, SeedableRng};
use serde::Serialize;

const MAX_CONSTRAINTS: usize = 10_000;
const MAX_ROUNDS: usize = 1_000;

/// Circuit parameters for a benchmark run.
#[derive(Clone, Copy, Debug, Serialize)]
pub struct BenchmarkConfig {
    pub constraints: usize,
    pub rounds: usize,
    pub seed: u64,
}

impl BenchmarkConfig {
    pub fn validate(&self) -> Result<(), String> {
        if self.constraints == 0 || self.constraints > MAX_CONSTRAINTS {
            return Err(format!(
                "constraints must be between 1 and {MAX_CONSTRAINTS}"
            ));
        }
        if self.rounds == 0 || self.rounds > MAX_ROUNDS {
            return Err(format!("rounds must be between 1 and {MAX_ROUNDS}"));
        }
        Ok(())
    }
}

/// A chain of `rounds` squaring constraints.
#[derive(Clone)]
pub struct MultiplicationCircuit {
    pub a: Option<Fr>,
    pub rounds: usize,
}

impl ConstraintSynthesizer<Fr> for MultiplicationCircuit {
    fn generate_constraints(self, cs: ConstraintSystemRef<Fr>) -> Result<(), SynthesisError> {
        let mut previous_value = self.a;
        let mut previous =
            cs.new_input_variable(|| self.a.ok_or(SynthesisError::AssignmentMissing))?;

        for _ in 0..self.rounds {
            let next_value = previous_value.map(|value| value.square());
            let next =
                cs.new_witness_variable(|| next_value.ok_or(SynthesisError::AssignmentMissing))?;

            cs.enforce_r1cs_constraint(|| lc!() + previous, || lc!() + previous, || lc!() + next)?;

            previous = next;
            previous_value = next_value;
        }

        Ok(())
    }
}

/// Latency summary in milliseconds.
#[derive(Clone, Debug, Serialize)]
pub struct TimingStats {
    pub min_ms: f64,
    pub mean_ms: f64,
    pub median_ms: f64,
    pub p95_ms: f64,
    pub max_ms: f64,
}

impl TimingStats {
    fn from_samples(mut samples: Vec<f64>) -> Result<Self, String> {
        if samples.is_empty() {
            return Err("no timing samples were collected".to_string());
        }
        let count = samples.len() as f64;
        let sum: f64 = samples.iter().sum();
        let mean_ms = sum / count;
        samples.sort_by(|a, b| a.total_cmp(b));
        let median_ms = if samples.len() % 2 == 0 {
            let upper = samples.len() / 2;
            (samples[upper - 1] + samples[upper]) / 2.0
        } else {
            samples[samples.len() / 2]
        };
        let p95_index = ((samples.len() * 95) / 100).min(samples.len() - 1);

        Ok(Self {
            min_ms: samples[0],
            mean_ms,
            median_ms,
            p95_ms: samples[p95_index],
            max_ms: samples[samples.len() - 1],
        })
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct CircuitMetrics {
    pub constraints: usize,
    pub public_inputs: usize,
    pub witnesses: usize,
}

#[derive(Clone, Debug, Serialize)]
pub struct BenchmarkReport {
    pub proof_system: &'static str,
    pub curve: &'static str,
    pub architecture: &'static str,
    pub operating_system: &'static str,
    pub config: BenchmarkConfig,
    pub circuit: CircuitMetrics,
    pub setup_ms: f64,
    pub prove: TimingStats,
    pub verify: TimingStats,
    pub proof_size_bytes: usize,
    pub proof_size_uncompressed_bytes: usize,
    pub verify_ok: bool,
    pub note: &'static str,
}

fn circuit_metrics(rounds: usize) -> Result<CircuitMetrics, String> {
    let cs = ConstraintSystem::<Fr>::new_ref();
    MultiplicationCircuit {
        a: Some(Fr::from(7_u64)),
        rounds,
    }
    .generate_constraints(cs.clone())
    .map_err(|error| format!("could not synthesize circuit: {error}"))?;

    Ok(CircuitMetrics {
        constraints: cs.num_constraints(),
        // Arkworks includes the constant `One` in the instance vector.
        public_inputs: cs.num_instance_variables().saturating_sub(1),
        witnesses: cs.num_witness_variables(),
    })
}

/// Run setup, proving, verification, and proof-size measurements.
pub fn run_benchmark(config: BenchmarkConfig) -> Result<BenchmarkReport, String> {
    config.validate()?;

    let circuit = circuit_metrics(config.constraints)?;
    let mut rng = StdRng::seed_from_u64(config.seed);

    let setup_start = Instant::now();
    let (proving_key, verifying_key) = Groth16::<Bls12_381>::setup(
        MultiplicationCircuit {
            a: None,
            rounds: config.constraints,
        },
        &mut rng,
    )
    .map_err(|error| format!("Groth16 setup failed: {error}"))?;
    let setup_ms = setup_start.elapsed().as_secs_f64() * 1_000.0;

    let processed_vk = prepare_verifying_key(&verifying_key);
    let mut prove_samples = Vec::with_capacity(config.rounds);
    let mut verify_samples = Vec::with_capacity(config.rounds);
    let mut proof_sizes = Vec::with_capacity(config.rounds);
    let mut all_verified = true;

    for round in 0..config.rounds {
        let a = Fr::from(7_u64 + round as u64);

        let prove_start = Instant::now();
        let proof = Groth16::<Bls12_381>::prove(
            &proving_key,
            MultiplicationCircuit {
                a: Some(a),
                rounds: config.constraints,
            },
            &mut rng,
        )
        .map_err(|error| format!("Groth16 proving failed: {error}"))?;
        prove_samples.push(prove_start.elapsed().as_secs_f64() * 1_000.0);

        proof_sizes.push(proof.serialized_size(Compress::Yes));

        let verify_start = Instant::now();
        let verified = Groth16::<Bls12_381>::verify_with_processed_vk(&processed_vk, &[a], &proof)
            .map_err(|error| format!("Groth16 verification failed: {error}"))?;
        verify_samples.push(verify_start.elapsed().as_secs_f64() * 1_000.0);
        all_verified &= verified;
    }

    if !all_verified {
        return Err("one or more proofs failed verification".to_string());
    }

    let proof_size_bytes = proof_sizes[0];
    let uncompressed_proof_size = {
        let proof = Groth16::<Bls12_381>::prove(
            &proving_key,
            MultiplicationCircuit {
                a: Some(Fr::from(7_u64)),
                rounds: config.constraints,
            },
            &mut rng,
        )
        .map_err(|error| format!("Groth16 proving failed: {error}"))?;
        proof.serialized_size(Compress::No)
    };

    Ok(BenchmarkReport {
        proof_system: "Groth16",
        curve: "BLS12-381",
        architecture: std::env::consts::ARCH,
        operating_system: std::env::consts::OS,
        config,
        circuit,
        setup_ms,
        prove: TimingStats::from_samples(prove_samples)?,
        verify: TimingStats::from_samples(verify_samples)?,
        proof_size_bytes,
        proof_size_uncompressed_bytes: uncompressed_proof_size,
        verify_ok: all_verified,
        note: "Benchmark only; this does not establish zero-knowledge, soundness, or production security.",
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn proves_and_verifies() {
        let report = run_benchmark(BenchmarkConfig {
            constraints: 8,
            rounds: 2,
            seed: 7,
        })
        .expect("benchmark should succeed");

        assert!(report.verify_ok);
        assert_eq!(report.circuit.constraints, 8);
        assert_eq!(report.circuit.public_inputs, 1);
        assert!(report.proof_size_bytes > 0);
        assert!(report.proof_size_bytes <= report.proof_size_uncompressed_bytes);
    }

    #[test]
    fn rejects_invalid_config() {
        assert!(run_benchmark(BenchmarkConfig {
            constraints: 0,
            rounds: 1,
            seed: 0,
        })
        .is_err());
        assert!(run_benchmark(BenchmarkConfig {
            constraints: 1,
            rounds: 0,
            seed: 0,
        })
        .is_err());
    }

    #[test]
    fn proof_size_is_constant_across_constraint_counts() {
        let small = run_benchmark(BenchmarkConfig {
            constraints: 1,
            rounds: 1,
            seed: 1,
        })
        .expect("small benchmark should succeed");
        let larger = run_benchmark(BenchmarkConfig {
            constraints: 20,
            rounds: 1,
            seed: 1,
        })
        .expect("larger benchmark should succeed");

        // Groth16 proofs are three group elements regardless of circuit size.
        assert_eq!(small.proof_size_bytes, larger.proof_size_bytes);
    }
}
