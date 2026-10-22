use std::env;
use std::process::ExitCode;

use zk_proof_performance_laboratory::{run_benchmark, BenchmarkConfig, BenchmarkReport};

fn usage() -> &'static str {
    "zk-bench [--constraints N] [--rounds N] [--seed N] [--json]\n\n\
     --constraints  number of squaring constraints (default: 100)\n\
     --rounds       prove/verify samples (default: 5)\n\
     --seed         deterministic setup RNG seed (default: 42)\n\
     --json         emit a machine-readable report\n\
     --help         show this message"
}

fn parse_value<T: std::str::FromStr>(flag: &str, value: Option<&String>) -> Result<T, String> {
    let value = value.ok_or_else(|| format!("{flag} requires a value"))?;
    value
        .parse()
        .map_err(|_| format!("{flag} has an invalid value: {value}"))
}

fn parse_args() -> Result<(BenchmarkConfig, bool), String> {
    let mut config = BenchmarkConfig {
        constraints: 100,
        rounds: 5,
        seed: 42,
    };
    let mut json = false;
    let mut args = env::args().skip(1);

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--constraints" => {
                config.constraints = parse_value("--constraints", args.next().as_ref())?
            }
            "--rounds" => config.rounds = parse_value("--rounds", args.next().as_ref())?,
            "--seed" => config.seed = parse_value("--seed", args.next().as_ref())?,
            "--json" => json = true,
            "--help" | "-h" => {
                println!("{}", usage());
                std::process::exit(0);
            }
            other => return Err(format!("unknown argument: {other}\n\n{}", usage())),
        }
    }

    Ok((config, json))
}

fn print_human(report: &BenchmarkReport) {
    println!("Proof system : {}", report.proof_system);
    println!("Curve        : {}", report.curve);
    println!(
        "Platform     : {} / {}",
        report.architecture, report.operating_system
    );
    println!(
        "Circuit      : {} constraints, {} public input(s), {} witnesses",
        report.circuit.constraints, report.circuit.public_inputs, report.circuit.witnesses
    );
    println!("Setup        : {:.3} ms", report.setup_ms);
    println!(
        "Prove        : min {:.3} ms | median {:.3} ms | mean {:.3} ms | p95 {:.3} ms | max {:.3} ms",
        report.prove.min_ms,
        report.prove.median_ms,
        report.prove.mean_ms,
        report.prove.p95_ms,
        report.prove.max_ms
    );
    println!(
        "Verify       : min {:.3} ms | median {:.3} ms | mean {:.3} ms | p95 {:.3} ms | max {:.3} ms",
        report.verify.min_ms,
        report.verify.median_ms,
        report.verify.mean_ms,
        report.verify.p95_ms,
        report.verify.max_ms
    );
    println!(
        "Proof size   : {} bytes compressed, {} bytes uncompressed",
        report.proof_size_bytes, report.proof_size_uncompressed_bytes
    );
    println!("Verified     : {}", report.verify_ok);
    println!("Note         : {}", report.note);
}

fn main() -> ExitCode {
    let result = (|| -> Result<(), String> {
        let (config, json) = parse_args()?;
        let report = run_benchmark(config)?;
        if json {
            println!(
                "{}",
                serde_json::to_string_pretty(&report)
                    .map_err(|error| format!("could not serialize report: {error}"))?
            );
        } else {
            print_human(&report);
        }
        Ok(())
    })();

    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("error: {error}");
            ExitCode::FAILURE
        }
    }
}
