use criterion::{black_box, criterion_group, criterion_main, Criterion};
use futures::future::join_all;
use std::sync::Arc;
use std::time::Duration;
use tokio::io::AsyncWriteExt;
use tokio::net::TcpStream;

use ipcow::{AddrData, AddrType, ListenerManager};

#[derive(Debug, Default)]
struct BenchMetrics {
    setup_time: Duration,
    connection_times: Vec<Duration>,
    throughput: f64,
}

async fn benchmark_connection_setup(ports: usize) -> BenchMetrics {
    let mut metrics = BenchMetrics::default();
    let start = std::time::Instant::now();

    let addr_data: Vec<AddrData> = (8000..8000 + ports)
        .map(|port| AddrData {
            info: AddrType::IPv4,
            socket_type: AddrType::TCP,
            address: (127, 0, 0, 1),
            port: port as u16,
        })
        .collect();

    let manager = ListenerManager::new(addr_data, ports.min(100)); // Cap concurrent connections

    // Spawn manager in background
    let manager_handle = tokio::spawn(async move {
        manager.run().await.unwrap();
    });

    // Quick connection test
    let connection_start = std::time::Instant::now();
    let semaphore = Arc::new(tokio::sync::Semaphore::new(100));

    let mut handles = Vec::new();
    for port in 8000..8000 + ports {
        let permit = semaphore.clone().acquire_owned().await.unwrap();
        handles.push(tokio::spawn(async move {
            if let Ok(mut stream) = TcpStream::connect(format!("127.0.0.1:{}", port)).await {
                stream.write_all(b"TEST\r\n").await.unwrap_or_default();
            }
            drop(permit);
        }));
    }

    join_all(handles).await;
    metrics.connection_times.push(connection_start.elapsed());
    manager_handle.abort();

    metrics.setup_time = start.elapsed();
    metrics.throughput = ports as f64 / metrics.connection_times[0].as_secs_f64();

    metrics
}

fn benchmark_server(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();

    let mut group = c.benchmark_group("server_performance");
    group
        .sample_size(10)
        .measurement_time(Duration::from_secs(20))
        .warm_up_time(Duration::from_secs(5));

    // Test different port counts
    for &ports in &[1, 10, 50, 100, 1000, 65000] {
        group.bench_function(format!("ports_{}", ports), |b| {
            b.to_async(&rt)
                .iter(|| async { black_box(benchmark_connection_setup(ports).await) });
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
