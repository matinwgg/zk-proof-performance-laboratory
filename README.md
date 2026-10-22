# Zero-Knowledge Proof Performance Laboratory

A reproducible Rust benchmark for **Groth16 over BLS12-381**. It measures setup,
proving, verification, proof size, and constraint scaling with a deterministic
circuit and a machine-readable JSON report.

This is a measurement tool, not a production proving service. Benchmark results
do not establish zero-knowledge, soundness, or cryptographic security.

## What is implemented

- A real Groth16 circuit built with the [arkworks](https://arkworks.rs/) stack
- One public input squared repeatedly; each squaring is one R1CS constraint
- Setup, prove, and verify timings: min, mean, median, p95, and max
- Compressed and uncompressed proof-size measurements
- Constraint, public-input, and witness counts
- Deterministic setup via a configurable RNG seed
- CLI with human-readable and `--json` output
- Criterion benchmark covering 1, 10, 100, and 1,000 constraints
- Unit tests for correctness, validation, and constant Groth16 proof size
- CI for formatting, clippy, tests, and benchmark compilation

## Project structure

```text
.
├── src/
│   ├── lib.rs          # circuit, benchmark harness, report types
│   └── main.rs         # CLI
├── benches/
│   └── groth16.rs      # Criterion prove/verify benchmarks
├── Cargo.toml
├── Cargo.lock
└── rust-toolchain.toml # Rust 1.89
```

## Requirements

- Rust 1.89 or newer (the pinned toolchain installs automatically with rustup)
- A C toolchain is not required for the default BLS12-381 build

## Run the benchmark

```bash
cargo run --release -- --constraints 100 --rounds 5
```

JSON report:

```bash
cargo run --release -- --constraints 100 --rounds 5 --json
```

Options:

```text
--constraints N   number of squaring constraints (default 100, max 10,000)
--rounds N        prove/verify samples (default 5, max 1,000)
--seed N          deterministic setup RNG seed (default 42)
--json            emit JSON instead of the human-readable summary
```

Example human-readable output (numbers depend on the machine):

```text
Proof system : Groth16
Curve        : BLS12-381
Platform     : aarch64 / linux
Circuit      : 100 constraints, 1 public input(s), 100 witnesses
Setup        : ...
Prove        : min ... ms | median ... ms | mean ... ms | p95 ... ms | max ... ms
Verify       : min ... ms | median ... ms | mean ... ms | p95 ... ms | max ... ms
Proof size   : 192 bytes compressed, 384 bytes uncompressed
Verified     : true
```

## Criterion benchmarks

```bash
cargo bench
```

Criterion writes HTML reports under `target/criterion/`. The benchmark covers
proving and verification for 1, 10, 100, and 1,000 constraints. Criterion is a
suite of microbenchmarks; use the CLI when you need one self-contained JSON
report with setup and proof-size measurements included.

## Methodology

- **Curve:** BLS12-381.
- **Proof system:** Groth16 with arkworks' libsnark reduction.
- **Circuit:** the public input `a` is repeatedly squared. Round `i` enforces
  `x_i = x_{i-1}^2`, with `x_0 = a`. The final value is not public; the proof
  demonstrates knowledge of the entire chain.
- **Constraint count:** one R1CS constraint per squaring. Arkworks may report
  additional bookkeeping variables, which the report includes explicitly.
- **Setup:** measured separately. It is circuit-specific and must not be reused
  across different circuits.
- **Timing:** wall-clock `std::time::Instant`, reported in milliseconds.
- **Proof size:** canonical serialization, compressed and uncompressed.
- **Determinism:** the setup RNG is seeded; proving samples use a deterministic
  sequence of public inputs. Timing itself is inherently noisy.

Groth16 proofs are three group elements, so **proof size is constant** as the
constraint count grows. Constraint count affects setup/proving time and key
size, not proof size. This is a protocol property, not a benchmark flaw.

## Interpreting results

Report the hardware, operating system, Rust version, curve, proof system,
constraint count, and sample count alongside results. Do not compare this
Groth16/BLS12-381 harness directly with a different proof system, curve, or
security parameter set without normalizing those assumptions.

The harness does not yet measure peak memory. Treat memory claims as out of
scope until a profiling pass is added.

## Security scope

This repository exercises a real proving library, but it does not provide an
audited circuit, trusted-setup ceremony, key management, or a production
verifier. Do not use the toy circuit as an application. Do not use benchmark
timings as evidence of cryptographic security.

## Development

```bash
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test
cargo bench --no-run
```

## Future work

- Add a second proof system and curve behind feature flags
- Measure peak memory with a profiler
- Report trusted-setup/key sizes
- Add CI-produced benchmark artifacts for a fixed runner class
- Add circuit benchmarks for realistic statements, not only a scaling toy

## License

MIT. See `LICENSE`.
