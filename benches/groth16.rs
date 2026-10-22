use std::hint::black_box;

use ark_bls12_381::{Bls12_381, Fr};
use ark_groth16::{prepare_verifying_key, Groth16};
use ark_snark::{CircuitSpecificSetupSNARK, SNARK};
use ark_std::rand::{rngs::StdRng, SeedableRng};
use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use zk_proof_performance_laboratory::MultiplicationCircuit;

fn benchmark_groth16(c: &mut Criterion) {
    let mut group = c.benchmark_group("groth16_bls12_381");
    group.sample_size(10);

    for constraints in [1_usize, 10, 100, 1_000] {
        let mut setup_rng = StdRng::seed_from_u64(42);
        let (proving_key, verifying_key) = Groth16::<Bls12_381>::setup(
            MultiplicationCircuit {
                a: None,
                rounds: constraints,
            },
            &mut setup_rng,
        )
        .expect("setup should succeed");
        let processed_vk = prepare_verifying_key(&verifying_key);
        let a = Fr::from(7_u64);
        let sample_proof = Groth16::<Bls12_381>::prove(
            &proving_key,
            MultiplicationCircuit {
                a: Some(a),
                rounds: constraints,
            },
            &mut StdRng::seed_from_u64(1),
        )
        .expect("sample proof should succeed");

        group.bench_with_input(
            BenchmarkId::new("prove", constraints),
            &constraints,
            |b, &rounds| {
                let mut rng = StdRng::seed_from_u64(1);
                b.iter(|| {
                    let proof = Groth16::<Bls12_381>::prove(
                        &proving_key,
                        MultiplicationCircuit { a: Some(a), rounds },
                        &mut rng,
                    )
                    .expect("proving should succeed");
                    black_box(proof);
                });
            },
        );

        group.bench_with_input(
            BenchmarkId::new("verify", constraints),
            &constraints,
            |b, _| {
                b.iter(|| {
                    let verified = Groth16::<Bls12_381>::verify_with_processed_vk(
                        &processed_vk,
                        &[a],
                        black_box(&sample_proof),
                    )
                    .expect("verification should succeed");
                    black_box(verified);
                });
            },
        );
    }

    group.finish();
}

criterion_group!(benches, benchmark_groth16);
criterion_main!(benches);
