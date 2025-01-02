use criterion::{black_box, criterion_group, criterion_main, Criterion};
use std::time::Duration;
use tokio::net::TcpStream;
use tokio::io::AsyncWriteExt;
use std::sync::Arc;
use futures::future::join_all;

use ipcow::{AddrType, AddrData, ListenerManager};

#[derive(Debug, Default)]
struct BenchMetrics {
    setup_time: Duration,
    connection_times: Vec<Duration>,
    throughput: f64,
}

async fn benchmark_connection_setup(ports: usize) -> BenchMetrics {
    let mut metrics = BenchMetrics::default();
    let start = std::time::Instant::now();

    // Create test address data
    let addr_data: Vec<AddrData> = (8000..8000+ports)
        .map(|port| AddrData {
            info: AddrType::IPv4,
            socket_type: AddrType::TCP,
            address: (127, 0, 0, 1),
            port: port as u16,
        })
        .collect();

    let _manager = ListenerManager::new(addr_data, ports);
    metrics.setup_time = start.elapsed();
    // Wait for listeners to be ready
    tokio::time::sleep(Duration::from_millis(100)).await;
    // Benchmark connection handling
    let connection_start = std::time::Instant::now();
    let max_concurrent = 100;
    let semaphore = Arc::new(tokio::sync::Semaphore::new(max_concurrent));
    
    let mut handles = Vec::new();
    for port in 8000..8000+ports {
        let permit = semaphore.clone().acquire_owned().await.unwrap();
        handles.push(tokio::spawn(async move {
            let addr = format!("127.0.0.1:{}", port);
            if let Ok(mut stream) = TcpStream::connect(&addr).await {
                let data = b"TEST DATA\r\n";
                if let Err(e) = stream.write_all(data).await {
                    eprintln!("Write error on {}: {}", addr, e);
                }
            }
            drop(permit);
        }));
    }

    join_all(handles).await;
    metrics.connection_times.push(connection_start.elapsed());
    
    // Calculate throughput
    let total_time = metrics.connection_times.iter().sum::<Duration>();
    metrics.throughput = ports as f64 / total_time.as_secs_f64();
    
    metrics
}

fn benchmark_server(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    
    let mut group = c.benchmark_group("server_performance");
    group.sample_size(10)
        .measurement_time(Duration::from_secs(20))
        .warm_up_time(Duration::from_secs(5));

    // Test different port counts
    for &ports in &[1, 10, 50, 100, 1000, 65000] {
        group.bench_function(format!("ports_{}", ports), |b| {
            b.to_async(&rt).iter(|| async {
                black_box(benchmark_connection_setup(ports).await)
            });
        });
    }

    group.finish();
}

criterion_group!(
    name = benches;
    config = Criterion::default()
        .sample_size(10)
        .warm_up_time(Duration::from_secs(5));
    targets = benchmark_server
);
criterion_main!(benches);