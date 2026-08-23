use std::time::{Duration,Instant};

#[derive(Debug,Clone,Copy)] pub struct Benchmark { pub rounds:u32, pub operations:u64, pub elapsed:Duration }

pub fn benchmark<F: FnMut()>(rounds:u32, operations:u64, mut f:F)->Benchmark { if rounds==0 || operations==0 { panic!("rounds and operations must be positive") } let start=Instant::now(); for _ in 0..rounds { for _ in 0..operations { f(); }} Benchmark{rounds,operations,elapsed:start.elapsed()} }

pub fn throughput(b:&Benchmark)->f64 { (b.rounds as f64 * b.operations as f64) / b.elapsed.as_secs_f64().max(f64::MIN_POSITIVE) }
