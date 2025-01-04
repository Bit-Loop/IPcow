use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::thread::{self, available_parallelism};
use std::num::NonZeroUsize;
use std::time::{Duration, Instant};
use sysinfo::{System, CpuRefreshKind, RefreshKind};
use rayon::prelude::*;
use std::sync::Mutex;

#[derive(Debug)]
struct BenchmarkResult {
    cpu_usage: f32,
    memory_usage: f64,
    io_throughput: f64,
    latency: Duration,
    cpu_tracker: Option<CpuTracker>,
}

#[derive(Debug)]
struct SystemMetrics {
    max_cpu_usage: f32,
    optimal_threads: usize,
    total_workers: usize,
    memory_usage_mb: f64,
    benchmark_duration: Duration,
}

#[derive(Debug)]
struct CpuSample {
    timestamp: Instant,
    usage: f32,
}

#[derive(Debug)]
struct CpuTracker {
    samples: Vec<f32>,
    peak: f32,
    rolling_avg: f32,
    sample_count: usize,
}

impl CpuTracker {
    fn new() -> Self {
        Self {
            samples: Vec::with_capacity(100),
            peak: 0.0,
            rolling_avg: 0.0,
            sample_count: 0,
        }
    }

    fn add_sample(&mut self, usage: f32) {
        if usage.is_nan() || usage <= 0.0 {
            eprintln!("Skipping invalid CPU usage sample: {}", usage);
            return;
        }
        
        self.samples.push(usage);
        self.peak = self.peak.max(usage);
        self.sample_count += 1;
        
        // Calculate rolling average over last 10 samples
        let window = self.samples.len().min(10);
        self.rolling_avg = self.samples.iter()
            .rev()
            .take(window)
            .sum::<f32>() / window as f32;
    }
}

#[derive(Debug)]
struct CpuMeasurement {
    timestamp: Instant,
    total_time: u64,
    idle_time: u64,
    per_core: Vec<f32>,
}

pub fn get_thread_factor() -> usize {
    let system_threads = available_parallelism()
        .unwrap_or(NonZeroUsize::new(1).unwrap())
        .get();
    
    let mut system = System::new_all();
    system.refresh_all();

    let base_workers = system_threads;
    let max_workers = base_workers * 4;
    
    let (optimal, metrics) = find_optimal_workers(&mut system, base_workers, max_workers);
    
    // Print detailed system metrics
    println!("\n=== System Performance Metrics ===");
    println!("Max CPU Usage: {:.1}%", metrics.max_cpu_usage);
    println!("System Threads: {}", system_threads);
    println!("Optimal Workers: {}", optimal);
    println!("Total Workers Tested: {}", metrics.total_workers);
    println!("Memory Usage: {:.1} MB", metrics.memory_usage_mb);
    println!("Benchmark Duration: {:?}", metrics.benchmark_duration);
    println!("===============================\n");

    optimal
}

fn calculate_memory_factor(sys: &System) -> f64 {
    let total_mem = sys.total_memory() as f64;
    let used_mem = sys.used_memory() as f64;
    
    // Scale factor based on available memory
    (1.0 - (used_mem / total_mem)).max(0.1)
}

fn calculate_cpu_factor(sys: &System) -> f64 {
    let cpu_load = sys.cpus().iter()
        .map(|cpu| cpu.cpu_usage() as f64)
        .sum::<f64>() / sys.cpus().len() as f64;
    
    // Inverse relationship with CPU load
    (100.0 - cpu_load) / 100.0
}

fn calculate_load_factor(cpu_available: f64, memory_available: f64) -> f64 {
    let cpu_weight = 0.7;
    let memory_weight = 0.3;
    
    (cpu_available * cpu_weight + memory_available * memory_weight)
        .clamp(0.1, 1.0)
}

fn calculate_max_safe_threads(sys: &System) -> usize {
    let memory_per_thread = 5_000_000f64; // 5MB per thread estimate
    let available_memory = sys.available_memory() as f64;
    let memory_limited_threads = (available_memory / memory_per_thread) as usize;
    
    let cpu_cores = sys.cpus().len();
    let cpu_limited_threads = cpu_cores * 2;
    
    std::cmp::min(memory_limited_threads, cpu_limited_threads)
}

fn find_optimal_workers(system: &mut System, base: usize, max: usize) -> (usize, SystemMetrics) {
    let mut best_workers = base;
    let mut best_score = 0.0;
    let start_time = Instant::now();
    let mut max_cpu: f32 = 0.0;
    let mut total_tested = 0;
    
    println!("\n=== Starting Worker Optimization ===");
    
    for workers in (base..=max).step_by(base) {
        println!("\nTesting with {} workers...", workers);
        let result = run_benchmark(workers, system);
        
        // Update metrics
        max_cpu = max_cpu.max(result.cpu_usage);
        total_tested += 1;
        
        // Print current metrics
        println!("Current CPU Usage: {:.1}%", result.cpu_usage);
        println!("Peak CPU Usage: {:.1}%", max_cpu);
        
        let score = calculate_efficiency_score(&result, workers);
        if score > best_score {
            best_score = score;
            best_workers = workers;
            println!("New best workers: {} (score: {:.2})", best_workers, best_score);
        }

        // Break if CPU usage is too high
        if result.cpu_usage > 90.0 {
            println!("CPU usage too high, stopping optimization");
            break;
        }

        // Allow system to stabilize
        thread::sleep(Duration::from_millis(500));
    }

    let metrics = SystemMetrics {
        max_cpu_usage: max_cpu,
        optimal_threads: best_workers,
        total_workers: total_tested,
        memory_usage_mb: system.used_memory() as f64 / 1024.0 / 1024.0,
        benchmark_duration: start_time.elapsed(),
    };

    (best_workers, metrics)
}

fn run_benchmark(workers: usize, system: &mut System) -> BenchmarkResult {
    let start = Instant::now();
    let ops_counter = Arc::new(AtomicU64::new(0));
    let cpu_samples = Arc::new(Mutex::new(Vec::new()));
    let mut cpu_tracker = CpuTracker::new();
    
    // Get initial CPU usage
    system.refresh_cpu_usage();
    let initial_cpu = system.global_cpu_usage();
    thread::sleep(Duration::from_millis(100));
    
    // CPU sampling thread with improved measurement
    let samples = Arc::clone(&cpu_samples);
    let sampler = thread::spawn(move || {
        let mut local_system = System::new_with_specifics(
            RefreshKind::default().with_cpu(CpuRefreshKind::everything())
        );
        while start.elapsed() < Duration::from_secs(4) {
            local_system.refresh_cpu_all();
            let usage = local_system.global_cpu_usage();
            if !usage.is_nan() && usage > 0.0 {
                samples.lock().unwrap().push(CpuSample {
                    timestamp: Instant::now(),
                    usage,
                });
            }
            thread::sleep(Duration::from_millis(100));
        }
    });

    let handles: Vec<_> = (0..workers)
        .map(|_| spawn_worker_thread(&ops_counter))
        .collect();

    for handle in handles {
        handle.join().unwrap();
    }
    sampler.join().unwrap();

    // Improved CPU usage calculation
    let samples = cpu_samples.lock().unwrap();
    let valid_samples: Vec<_> = samples.iter()
        .skip(5)
        .filter(|s| s.usage > 0.0 && !s.usage.is_nan())
        .collect();

    let peak_cpu = valid_samples.iter()
        .map(|s| s.usage)
        .max_by(|a, b| a.partial_cmp(b).unwrap())
        .unwrap_or(0.0);

    let avg_cpu = if !valid_samples.is_empty() {
        valid_samples.iter()
            .map(|s| s.usage)
            .sum::<f32>() / valid_samples.len() as f32
    } else {
        0.0
    };

    for sample in valid_samples {
        cpu_tracker.add_sample(sample.usage);
    }

    system.refresh_memory();
    BenchmarkResult {
        cpu_usage: peak_cpu.max(avg_cpu),
        memory_usage: system.used_memory() as f64,
        io_throughput: ops_counter.load(Ordering::Relaxed) as f64 / 3.0,
        latency: start.elapsed(),
        cpu_tracker: Some(cpu_tracker),
    }
}

fn calculate_cpu_usage(measurements: &[CpuMeasurement]) -> f32 {
    if measurements.len() < 2 {
        return 0.0;
    }

    let valid_samples: Vec<f32> = measurements
        .windows(2)
        .map(|window| {
            let [prev, curr] = window else { return 0.0 };
            let total_delta = curr.total_time.saturating_sub(prev.total_time);
            let idle_delta = curr.idle_time.saturating_sub(prev.idle_time);
            if total_delta == 0 {
                return 0.0;
            }
            ((total_delta - idle_delta) as f32 / total_delta as f32) * 100.0
        })
        .filter(|&usage| usage > 5.0 && !usage.is_nan())
        .collect();

    if valid_samples.is_empty() {
        return 0.0;
    }

    let window = {
        let window_size = (valid_samples.len() as f32 * 0.2) as usize;
        window_size.clamp(2, 10)
    };

    valid_samples.iter().rev().take(window).sum::<f32>() / window as f32
}

/// Calculate efficiency score based on multiple metrics
fn calculate_efficiency_score(result: &BenchmarkResult, workers: usize) -> f64 {
    let cpu_tracker = result.cpu_tracker.as_ref().unwrap();
    
    // Score based on peak CPU usage
    let cpu_efficiency = if cpu_tracker.peak > 90.0 {
        0.0 // Overloaded
    } else if cpu_tracker.peak > 70.0 {
        0.5 // High load
    } else {
        1.0 // Optimal load
    };

    // Score based on rolling average
    let stability_score = if cpu_tracker.rolling_avg < 50.0 {
        0.5 // Underutilized
    } else if cpu_tracker.rolling_avg < 80.0 {
        1.0 // Good utilization
    } else {
        0.3 // Too high
    };

    (cpu_efficiency + stability_score) / 2.0
}

/// Calculate optimal workers based on benchmark results and system capabilities
pub fn calculate_optimal_workers(max_workers: usize) -> usize {
    let mut system = System::new_all();
    let base_workers = available_parallelism()
        .unwrap_or(NonZeroUsize::new(1).unwrap())
        .get();

    find_optimal_workers(&mut system, base_workers, max_workers).0
}

fn spawn_worker_thread(ops_counter: &Arc<AtomicU64>) -> thread::JoinHandle<()> {
    let ops_counter = Arc::clone(ops_counter);
    thread::spawn(move || {
        let worker_start = Instant::now();
        while worker_start.elapsed().as_secs() < 3 {
            // More intensive CPU work
            let _: f64 = (0..25_000)
                .into_par_iter()
                .map(|x| (x as f64).sqrt().sin().cos().powi(2))
                .sum();
            ops_counter.fetch_add(1, Ordering::Relaxed);
        }
    })
}

fn create_cpu_tracker(measurements: &[CpuMeasurement]) -> CpuTracker {
    let mut cpu_tracker = CpuTracker::new();
    for measurement in measurements {
        for &usage in &measurement.per_core {
            cpu_tracker.add_sample(usage);
        }
    }
    cpu_tracker
}
