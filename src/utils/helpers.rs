use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::thread::{self, available_parallelism};
use std::num::NonZeroUsize;
use std::time::{Duration, Instant};
use sysinfo::{System, CpuRefreshKind, RefreshKind};
use rayon::prelude::*;
use std::sync::Mutex;
use tokio::net::{TcpListener, TcpStream};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use std::net::SocketAddr;
use futures::stream::{self, StreamExt};
use tokio::time::sleep;

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
    let max_workers = base_workers * 16; // Increased max multiplier
    
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
    let mut last_cpu = 0.0;
    
    let phase = "Ramp";
    
    print!("\x1B[2J\x1B[1;1H");
    println!("=== Worker Optimization in Progress ===\n");
    
    let mut workers = base;
    while workers <= max {
        print!("\x1B[2K");
        println!("\rPhase: {} | Workers: {} | Calculating...", phase, workers);
        
        let result = run_benchmark(workers, system);
        max_cpu = max_cpu.max(result.cpu_usage);
        total_tested += 1;
        
        print!("\x1B[1A\x1B[2K");
        println!(
            "\r{} | Workers: {} | CPU: {:.1}% | Peak: {:.1}% | Score: {:.2}",
            phase, workers, result.cpu_usage, max_cpu,
            calculate_efficiency_score(&result, workers)
        );
        
        let score = calculate_efficiency_score(&result, workers);
        if score > best_score {
            best_score = score;
            best_workers = workers;
            println!("► New best configuration found!");
        }

        if result.cpu_usage >= 65.0 {
            // Reduce worker count before fine tuning
            workers = (workers as f32 * 0.7) as usize;
            println!("► Starting fine-tune phase from {} workers", workers);
            break;
        }
        
        workers = if result.cpu_usage < 30.0 {
            (workers as f32 * 1.5) as usize // More gradual scaling
        } else if result.cpu_usage < 50.0 {
            (workers as f32 * 1.3) as usize
        } else {
            (workers as f32 * 1.2) as usize
        };

        workers = workers.min(max);
        thread::sleep(Duration::from_millis(250)); // Increased stabilization time
        last_cpu = result.cpu_usage;
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

    // Spawn worker threads that simulate actual server workload
    let handles: Vec<_> = (0..workers)
        .map(|_| spawn_realistic_worker_thread(&ops_counter))
        .collect();

    for handle in handles {
        handle.join().unwrap();
    }

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
    
    // CPU utilization score (0.0 - 1.0)
    let cpu_score = if cpu_tracker.peak > 95.0 {
        0.0 // Overloaded
    } else if cpu_tracker.peak > 85.0 {
        0.3 // Very high
    } else if cpu_tracker.peak > 75.0 {
        0.8 // Near optimal
    } else if cpu_tracker.peak > 65.0 {
        1.0 // Optimal
    } else if cpu_tracker.peak > 50.0 {
        0.7 // Moderate
    } else {
        0.4 // Underutilized
    };

    // Stability score based on variance between peak and average
    let stability_score = {
        let variance = (cpu_tracker.peak - cpu_tracker.rolling_avg).abs();
        if variance < 5.0 { 1.0 }
        else if variance < 10.0 { 0.8 }
        else if variance < 15.0 { 0.6 }
        else if variance < 20.0 { 0.4 }
        else { 0.2 }
    };

    // Throughput efficiency (workers vs CPU usage ratio)
    let throughput_score = {
        let cpu_per_worker = cpu_tracker.rolling_avg / workers as f32;
        if cpu_per_worker > 2.0 { 0.3 } // Too few workers
        else if cpu_per_worker > 1.0 { 0.6 }
        else if cpu_per_worker > 0.5 { 1.0 } // Optimal ratio
        else if cpu_per_worker > 0.2 { 0.7 }
        else { 0.4 } // Too many workers
    };

    // Weighted combination of scores
    (cpu_score * 0.5 + stability_score * 0.3 + throughput_score * 0.2)
}

/// Calculate optimal workers based on benchmark results and system capabilities
pub fn calculate_optimal_workers(max_workers: usize) -> usize {
    let mut system = System::new_all();
    let base_workers = available_parallelism()
        .unwrap_or(NonZeroUsize::new(1).unwrap())
        .get();

    find_optimal_workers(&mut system, base_workers, max_workers).0
}

fn spawn_realistic_worker_thread(ops_counter: &Arc<AtomicU64>) -> thread::JoinHandle<()> {
    let ops_counter = Arc::clone(ops_counter);
    thread::spawn(move || {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_io()
            .enable_time()
            .build()
            .unwrap();

        runtime.block_on(async {
            // Find an available port
            let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
            let addr = listener.local_addr().unwrap();
            
            // Spawn echo server
            let server = tokio::spawn(async move {
                while let Ok((mut socket, _)) = listener.accept().await {
                    tokio::spawn(async move {
                        let mut buf = [0u8; 512]; // Reduced buffer size
                        while let Ok(n) = socket.read(&mut buf).await {
                            if n == 0 { break; }
                            socket.write_all(&buf[..n]).await.unwrap();
                            // Add small delay between operations
                            tokio::time::sleep(Duration::from_micros(500)).await;
                        }
                    });
                }
            });

            // Client workload with throttling
            let start = Instant::now();
            while start.elapsed().as_secs() < 3 {
                if let Ok(mut stream) = TcpStream::connect(addr).await {
                    let data = vec![1u8; 512]; // Reduced packet size
                    stream.write_all(&data).await.unwrap();
                    let mut response = vec![0u8; 512];
                    stream.read_exact(&mut response).await.unwrap();
                    ops_counter.fetch_add(1, Ordering::Relaxed);
                    // Add delay between connections
                    tokio::time::sleep(Duration::from_millis(5)).await;
                }
            }
            drop(server);
        });
    })
}

// Helper functions to simulate actual server workload
fn process_mock_request(data: &[u8]) -> Vec<u8> {
    // Simulate HTTP request parsing and response generation
    let mut response = Vec::with_capacity(data.len());
    for &byte in data.iter() {
        response.push(byte.wrapping_add(1));
    }
    
    // Simulate HTTP header generation
    response.extend_from_slice(b"HTTP/1.1 200 OK\r\n");
    response
}

fn analyze_mock_service(data: &[u8]) -> String {
    // Simulate service fingerprinting
    let mut hash = 0u64;
    for &byte in data {
        hash = hash.wrapping_mul(31).wrapping_add(byte as u64);
    }
    format!("Service-{:x}", hash)
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
