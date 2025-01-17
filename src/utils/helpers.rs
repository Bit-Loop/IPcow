use futures::stream::{self, StreamExt};
use std::fs::{File, OpenOptions};
use std::io::{self, BufWriter};
use std::io::{BufRead, BufReader, Write};
use std::net::SocketAddr;
use std::num::NonZeroUsize;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::sync::Mutex;
use std::thread::{self, available_parallelism};
use std::time::{Duration, Instant};
use sysinfo::{CpuRefreshKind, RefreshKind, System};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::time::sleep;

#[derive(Debug)]
struct BenchmarkResult {
    cpu_usage: f32,
    memory_usage: f64,
    io_throughput: f64,
    latency: Duration,
    cpu_tracker: Option<CpuTracker>,
    total_tasks: u64,   // Add total tasks counter
    total_threads: u64, // Add total threads counter
}

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
struct SystemMetrics {
    max_cpu_usage: f32,
    optimal_threads: usize,
    total_workers: usize,
    memory_usage_mb: f64,
    #[serde(skip)]
    benchmark_duration: Duration,
    total_tasks: u64,   // Add total tasks counter
    total_threads: u64, // Add total threads counter
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
        self.rolling_avg = self.samples.iter().rev().take(window).sum::<f32>() / window as f32;
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
    // Check for existing metrics on disk
    if let Ok(metrics) = read_metrics_from_file() {
        println!("Metrics loaded from file: {:?}", metrics);
        return metrics.optimal_threads;
    }

    let system_threads = available_parallelism()
        .unwrap_or(NonZeroUsize::new(1).unwrap())
        .get();

    let mut system = System::new_all();
    system.refresh_all();

    let base_workers = system_threads;
    let max_workers = base_workers * 32; // Doubled from 16 to allow more headroom

    let (optimal, metrics) = find_optimal_workers(&mut system, base_workers, max_workers);

    // Print detailed system metrics
    println!("\n=== System Performance Metrics ===");
    println!("Max CPU Usage: {:.1}%", metrics.max_cpu_usage);
    println!("System Threads: {}", system_threads);
    println!("Optimal Workers: {}", optimal);
    println!("Total Workers Tested: {}", metrics.total_workers);
    println!("Total Tasks Run: {}", metrics.total_tasks);
    println!("Total Threads Created: {}", metrics.total_threads);
    println!("Memory Usage: {:.1} MB", metrics.memory_usage_mb);
    println!("Benchmark Duration: {:?}", metrics.benchmark_duration);
    println!("===============================\n");

    // Write metrics to file
    write_metrics_to_file(&metrics).expect("Failed to write metrics to file");

    optimal
}

fn calculate_memory_factor(sys: &System) -> f64 {
    let total_mem = sys.total_memory() as f64;
    let used_mem = sys.used_memory() as f64;

    // Scale factor based on available memory
    (1.0 - (used_mem / total_mem)).max(0.1)
}

fn calculate_cpu_factor(sys: &System) -> f64 {
    let cpu_load = sys
        .cpus()
        .iter()
        .map(|cpu| cpu.cpu_usage() as f64)
        .sum::<f64>()
        / sys.cpus().len() as f64;

    // Inverse relationship with CPU load
    (100.0 - cpu_load) / 100.0
}

fn calculate_load_factor(cpu_available: f64, memory_available: f64) -> f64 {
    let cpu_weight = 0.7;
    let memory_weight = 0.3;

    (cpu_available * cpu_weight + memory_available * memory_weight).clamp(0.1, 1.0)
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
    let mut optimal_cpu = 0.0;
    let start_time = Instant::now();
    let mut max_cpu: f32 = 0.0;
    let mut total_tested = 0;
    let mut last_cpu = 0.0;
    let mut plateau_counter = 0;
    let target_cpu = 80.0; // Changed target CPU utilization to 80%
    let mut total_tasks = 0;
    let mut total_threads = 0;
    let mut last_improvement = Instant::now();

    println!("=== Worker Optimization in Progress ===\n");
    println!("Target CPU Utilization: {:.1}%\n", target_cpu);

    let mut next_workers = base;

    // Increase waiting time before the initial warm-up phase
    thread::sleep(Duration::from_secs(5));

    // Initial warm-up
    system.refresh_all();
    thread::sleep(Duration::from_millis(50));

    while next_workers <= max && start_time.elapsed() < Duration::from_secs(15) {
        let workers = next_workers;
        let result = run_benchmark(workers, system);

        total_tasks += result.total_tasks;
        total_threads += result.total_threads;
        max_cpu = max_cpu.max(result.cpu_usage);
        total_tested += 1;

        // Calculate scaling factors
        let cpu_percentage = (result.cpu_usage / target_cpu) * 100.0;
        let distance_factor = ((target_cpu - result.cpu_usage) / target_cpu).max(0.1);

        // Improved dynamic scaling based on current CPU usage and target
        next_workers = if result.cpu_usage < target_cpu {
            let scale = if distance_factor > 0.8 {
                4.0 // Very far from target (< 20% of target)
            } else if distance_factor > 0.6 {
                3.0 // Far from target (20-40% of target)
            } else if distance_factor > 0.4 {
                2.0 // Moderate distance (40-60% of target)
            } else if distance_factor > 0.2 {
                1.5 // Getting closer (60-80% of target)
            } else {
                1.2 // Close to target (80-100% of target)
            };

            (workers as f32 * scale) as usize
        } else {
            // Fine-tuning when we're above target
            (workers as f32 * 0.9) as usize
        };

        println!(
            "{} | Workers: {} | CPU: {:.1}% | Target: {:.1}% | Progress: {:.1}% | Scale: {:.1}x",
            if cpu_percentage < 90.0 {
                "Ramp"
            } else {
                "Fine-Tune"
            },
            workers,
            result.cpu_usage,
            target_cpu,
            cpu_percentage,
            (next_workers as f32 / workers as f32)
        );

        let score = calculate_efficiency_score(&result, workers);
        if score > best_score || (score >= best_score && result.cpu_usage > optimal_cpu) {
            best_score = score;
            best_workers = workers;
            optimal_cpu = result.cpu_usage;
            last_improvement = Instant::now();
            println!(
                "► New best configuration found! Workers: {} | CPU: {:.1}%",
                best_workers, optimal_cpu
            );
        }

        // Break conditions
        if last_improvement.elapsed() > Duration::from_secs(5) && total_tested > 4 {
            println!("► Optimization complete: no improvement for 5 seconds");
            break;
        }

        // Prevent getting stuck
        if next_workers == workers {
            next_workers = workers + (workers / 3);
            println!("► Breaking plateau - increasing workers by 33%");
        }

        next_workers = next_workers.min(max);
        last_cpu = result.cpu_usage;
        thread::sleep(Duration::from_millis(5));
    }

    // Rapid fine-tune phase
    if next_workers <= max && start_time.elapsed() < Duration::from_secs(15) {
        let workers = next_workers;
        let result = run_benchmark(workers, system);

        total_tasks += result.total_tasks;
        total_threads += result.total_threads;
        max_cpu = max_cpu.max(result.cpu_usage);
        total_tested += 1;

        // Fine-tuning adjustments
        next_workers = if result.cpu_usage < target_cpu {
            workers + 2
        } else {
            workers - 2
        };

        println!(
            "Fine-Tune | Workers: {} | CPU: {:.1}% | Target: {:.1}% | Progress: {:.1}%",
            workers,
            result.cpu_usage,
            target_cpu,
            (result.cpu_usage / target_cpu) * 100.0
        );

        let score = calculate_efficiency_score(&result, workers);
        if score > best_score || (score >= best_score && result.cpu_usage > optimal_cpu) {
            best_score = score;
            best_workers = workers;
            optimal_cpu = result.cpu_usage;
            last_improvement = Instant::now();
            println!(
                "► New best configuration found! Workers: {} | CPU: {:.1}%",
                best_workers, optimal_cpu
            );
        }

        // Break after the first fine-tune iteration
        println!("► First fine-tune iteration complete, stopping optimization.");
    }

    let metrics = SystemMetrics {
        max_cpu_usage: max_cpu,
        optimal_threads: best_workers,
        total_workers: total_tested,
        memory_usage_mb: system.used_memory() as f64 / 1024.0 / 1024.0,
        benchmark_duration: start_time.elapsed(),
        total_tasks,
        total_threads,
    };

    // Write metrics to file
    write_metrics_to_file(&metrics).expect("Failed to write metrics to file");

    (best_workers, metrics)
}

fn run_benchmark(workers: usize, system: &mut System) -> BenchmarkResult {
    let start = Instant::now();
    let ops_counter = Arc::new(AtomicU64::new(0));
    let task_counter = Arc::new(AtomicU64::new(0));
    let thread_counter = Arc::new(AtomicU64::new(0));
    let cpu_samples = Arc::new(Mutex::new(Vec::<CpuSample>::new()));
    let mut cpu_tracker = CpuTracker::new();

    // Warm-up phase
    system.refresh_all();
    thread::sleep(Duration::from_millis(100)); // Reduced warm-up time
    system.refresh_cpu_all();

    // Count main workers
    thread_counter.fetch_add(workers as u64, Ordering::SeqCst);

    // CPU sampling setup
    let samples = Arc::clone(&cpu_samples);
    let sampler = thread::spawn(move || {
        let mut local_system = System::new_with_specifics(
            RefreshKind::default().with_cpu(CpuRefreshKind::everything()),
        );

        // Initial warm-up sample
        local_system.refresh_cpu_all();
        thread::sleep(Duration::from_millis(50));

        while start.elapsed() < Duration::from_secs(1) {
            // Reduced from 2s to 1s
            local_system.refresh_cpu_all();
            let usage = local_system.global_cpu_usage();
            if !usage.is_nan() && usage > 0.0 {
                samples.lock().unwrap().push(CpuSample {
                    timestamp: Instant::now(),
                    usage,
                });
            }
            thread::sleep(Duration::from_millis(10)); // Reduced from 50ms to 10ms
        }
    });

    // Spawn worker threads
    let handles: Vec<_> = (0..workers)
        .map(|_| {
            let ops = Arc::clone(&ops_counter);
            let tasks = Arc::clone(&task_counter);
            let threads = Arc::clone(&thread_counter);

            thread::spawn(move || {
                let runtime = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .unwrap();

                runtime.block_on(async {
                    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
                    let addr = listener.local_addr().unwrap();

                    // Server task counter
                    let server_tasks = Arc::clone(&tasks);

                    let server = tokio::spawn(async move {
                        while let Ok((mut socket, _)) = listener.accept().await {
                            server_tasks.fetch_add(1, Ordering::SeqCst);

                            let handler_tasks = Arc::clone(&server_tasks);
                            tokio::spawn(async move {
                                let mut buf = vec![0; 4096];
                                loop {
                                    match socket.read(&mut buf).await {
                                        Ok(0) => break,
                                        Ok(n) => {
                                            if let Ok(request) =
                                                String::from_utf8(buf[..n].to_vec())
                                            {
                                                if request.starts_with("GET")
                                                    || request.starts_with("POST")
                                                {
                                                    let response =
                                                        process_mock_request(request.as_bytes());
                                                    if socket.write_all(&response).await.is_err() {
                                                        break;
                                                    }
                                                    handler_tasks.fetch_add(1, Ordering::SeqCst);
                                                }
                                            }
                                        }
                                        Err(_) => break,
                                    }
                                }
                            });
                        }
                    });

                    // Client task counter
                    let client_tasks = Arc::clone(&tasks);

                    while start.elapsed().as_secs() < 3 {
                        if let Ok(mut stream) = TcpStream::connect(addr).await {
                            client_tasks.fetch_add(1, Ordering::SeqCst);
                            // Send HTTP GET request with headers
                            let request = format!(
                                "GET / HTTP/1.1\r\n\
                                 Host: localhost\r\n\
                                 User-Agent: IPCow-Benchmark\r\n\
                                 Accept: */*\r\n\
                                 Connection: keep-alive\r\n\r\n"
                            );

                            if stream.write_all(request.as_bytes()).await.is_ok() {
                                let mut response = vec![0; 4096];
                                if let Ok(n) = stream.read(&mut response).await {
                                    if n > 0
                                        && String::from_utf8_lossy(&response[..n])
                                            .starts_with("HTTP/1.1")
                                    {
                                        ops.fetch_add(1, Ordering::SeqCst);
                                    }
                                }
                            }
                            tokio::time::sleep(Duration::from_millis(5)).await; // Reduced sleep time
                        }
                    }
                    drop(server);
                })
            })
        })
        .collect();

    // Wait for completion
    for handle in handles {
        handle.join().unwrap();
    }
    sampler.join().unwrap();

    // Improved CPU usage calculation
    let samples = cpu_samples.lock().unwrap();
    let valid_samples: Vec<_> = samples
        .iter()
        .skip(5)
        .filter(|s| s.usage > 0.0 && !s.usage.is_nan())
        .collect();

    let peak_cpu = valid_samples
        .iter()
        .map(|s| s.usage)
        .max_by(|a, b| a.partial_cmp(b).unwrap())
        .unwrap_or(0.0);

    let avg_cpu = if !valid_samples.is_empty() {
        valid_samples.iter().map(|s| s.usage).sum::<f32>() / valid_samples.len() as f32
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
        total_tasks: task_counter.load(Ordering::SeqCst),
        total_threads: thread_counter.load(Ordering::SeqCst),
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
    let cpu_score = if cpu_tracker.peak > 75.0 {
        1.0
    } else if cpu_tracker.peak > 65.0 {
        0.8
    } else if cpu_tracker.peak > 50.0 {
        0.7
    } else {
        0.4
    };

    // Stability score based on variance between peak and average
    let stability_score = {
        let variance = (cpu_tracker.peak - cpu_tracker.rolling_avg).abs();
        if variance < 5.0 {
            1.0
        } else if variance < 10.0 {
            0.8
        } else if variance < 15.0 {
            0.6
        } else if variance < 20.0 {
            0.4
        } else {
            0.2
        }
    };

    // Throughput efficiency (workers vs CPU usage ratio)
    let throughput_score = {
        let cpu_per_worker = cpu_tracker.rolling_avg / workers as f32;
        if cpu_per_worker > 2.0 {
            0.3
        }
        // Too few workers
        else if cpu_per_worker > 1.0 {
            0.6
        } else if cpu_per_worker > 0.5 {
            1.0
        }
        // Optimal ratio
        else if cpu_per_worker > 0.2 {
            0.7
        } else {
            0.4
        } // Too many workers
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

fn spawn_realistic_worker_thread(
    ops_counter: &Arc<AtomicU64>,
    task_counter: &Arc<AtomicU64>,
    thread_counter: &Arc<AtomicU64>,
) -> thread::JoinHandle<()> {
    let ops_counter = Arc::clone(ops_counter);
    let task_counter = Arc::clone(task_counter);
    let thread_counter = Arc::clone(thread_counter);

    thread_counter.fetch_add(1, Ordering::SeqCst);

    thread::spawn(move || {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();

        runtime.block_on(async {
            let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
            let addr = listener.local_addr().unwrap();

            // Server task with its own counter clones
            let server_ops = Arc::clone(&ops_counter);
            let server_task = Arc::clone(&task_counter);
            let server_thread = Arc::clone(&thread_counter);

            let server = tokio::spawn(async move {
                while let Ok((mut socket, _)) = listener.accept().await {
                    server_task.fetch_add(1, Ordering::SeqCst);

                    // Clone counters for each connection handler
                    let handler_ops = Arc::clone(&server_ops);
                    let handler_thread = Arc::clone(&server_thread);

                    handler_thread.fetch_add(1, Ordering::SeqCst);

                    tokio::spawn(async move {
                        let mut buf = vec![0; 4096];
                        loop {
                            match socket.read(&mut buf).await {
                                Ok(0) => break,
                                Ok(n) => {
                                    if let Ok(request) = String::from_utf8(buf[..n].to_vec()) {
                                        if request.starts_with("GET") || request.starts_with("POST")
                                        {
                                            let response = process_mock_request(request.as_bytes());
                                            if socket.write_all(&response).await.is_err() {
                                                break;
                                            }
                                            handler_ops.fetch_add(1, Ordering::SeqCst);
                                        }
                                    }
                                }
                                Err(_) => break,
                            }
                        }
                        handler_thread.fetch_sub(1, Ordering::SeqCst);
                    });
                }
            });

            // Client task with its own counter clone
            let client_ops = Arc::clone(&ops_counter);

            let start = Instant::now();
            while start.elapsed().as_secs() < 3 {
                if let Ok(mut stream) = TcpStream::connect(addr).await {
                    let request = format!(
                        "GET / HTTP/1.1\r\n\
                         Host: localhost\r\n\
                         User-Agent: IPCow-Benchmark\r\n\
                         Accept: */*\r\n\
                         Connection: keep-alive\r\n\r\n"
                    );

                    if stream.write_all(request.as_bytes()).await.is_ok() {
                        let mut response = vec![0; 4096];
                        if let Ok(n) = stream.read(&mut response).await {
                            if n > 0
                                && String::from_utf8_lossy(&response[..n]).starts_with("HTTP/1.1")
                            {
                                client_ops.fetch_add(1, Ordering::SeqCst);
                            }
                        }
                    }
                    tokio::time::sleep(Duration::from_millis(1)).await;
                }
            }
            drop(server);
        });
    })
}

fn process_mock_request(data: &[u8]) -> Vec<u8> {
    // Parse incoming request (simplified)
    let request = String::from_utf8_lossy(data);
    let is_get = request.starts_with("GET");
    let is_post = request.starts_with("POST");

    // Generate response with proper HTTP headers
    let body = if is_get {
        "Welcome to IPCow Benchmark Server"
    } else if is_post {
        "Received POST Request"
    } else {
        "Unknown Request Type"
    };

    // Current timestamp for headers
    let timestamp = chrono::Local::now().format("%a, %d %b %Y %H:%M:%S GMT");

    // Construct full HTTP response with headers
    format!(
        "HTTP/1.1 200 OK\r\n\
         Date: {}\r\n\
         Server: IPCow-Benchmark\r\n\
         Content-Type: text/plain\r\n\
         Content-Length: {}\r\n\
         Connection: keep-alive\r\n\
         \r\n\
         {}",
        timestamp,
        body.len(),
        body
    )
    .into_bytes()
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

fn write_metrics_to_file(metrics: &SystemMetrics) -> io::Result<()> {
    let file = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open("metrics.txt")?;
    let mut writer = BufWriter::new(file);
    let metrics_json = serde_json::to_string(metrics)?;
    let current_dir = std::env::current_dir().unwrap_or_default();
    println!(
        "Saving metrics to: {}",
        current_dir.join("metrics.txt").display()
    );
    writeln!(writer, "{}", metrics_json)?;
    Ok(())
}

fn read_metrics_from_file() -> io::Result<SystemMetrics> {
    let file = File::open("metrics.txt")?;
    let reader = BufReader::new(file);
    let mut lines = reader.lines();
    let current_dir = std::env::current_dir().unwrap_or_default();
    println!(
        "Loading metrics from: {}",
        current_dir.join("metrics.txt").display()
    );
    if let Some(line) = lines.next() {
        let metrics: SystemMetrics = serde_json::from_str(&line?)?;
        Ok(metrics)
    } else {
        Err(io::Error::new(io::ErrorKind::NotFound, "No metrics found"))
    }
}
