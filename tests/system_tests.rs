use ipcow::{AddrData, AddrType, ListenerManager};
use std::thread;
use std::time::Duration;
use sysinfo::{RefreshKind, System};
use tokio::runtime::Runtime;

#[test]
fn test_system_resources() {
    let mut sys = System::new_with_specifics(
        RefreshKind::everything(), // Changed from new() to everything()
    );
    sys.refresh_all();

    // Test memory usage
    assert!(
        sys.total_memory() > 0,
        "Total memory should be greater than 0"
    );
    assert!(
        sys.used_memory() <= sys.total_memory(),
        "Used memory should not exceed total memory"
    );

    // Test CPU usage over time
    for _ in 0..3 {
        sys.refresh_cpu_all(); // Changed from refresh_cpu() to refresh_cpu_all()
        for cpu in sys.cpus() {
            let usage = cpu.cpu_usage();
            assert!(
                usage >= 0.0 && usage <= 95.0,
                "CPU {} usage {:.1}% should be between 0% and 95%",
                cpu.name(),
                usage
            );
        }
        thread::sleep(Duration::from_millis(100));
    }

    // Test system info
    assert!(
        !System::host_name().unwrap_or_default().is_empty(),
        "Hostname should be available"
    ); // Changed to static method call
    assert!(
        !System::kernel_version().unwrap_or_default().is_empty(),
        "Kernel version should be available"
    ); // Changed to static method call
}

#[test]
fn test_process_resources() {
    let mut sys = System::new_all();
    sys.refresh_all();

    let current_pid = sysinfo::get_current_pid().expect("Should get current PID");
    let process = sys
        .process(current_pid)
        .expect("Should find current process");

    assert!(process.memory() > 0, "Process should use some memory");
    assert!(
        process.cpu_usage() >= 0.0,
        "CPU usage should be non-negative"
    );
}

#[test]
fn test_system_resources_with_server() {
    let rt = Runtime::new().unwrap();
    let mut sys = System::new_all();

    // Setup server
    let addr_data = vec![AddrData {
        info: AddrType::IPv4,
        socket_type: AddrType::TCP,
        address: (127, 0, 0, 1),
        port: 8080,
    }];

    let manager = ListenerManager::new(addr_data, 4);

    // Start server in background
    let server_handle = rt.spawn(async move {
        manager.run().await.unwrap();
    });

    // Monitor resources
    for _ in 0..3 {
        sys.refresh_all();

        // Memory checks
        assert!(
            sys.total_memory() > 0,
            "Total memory should be greater than 0"
        );
        assert!(
            sys.used_memory() <= sys.total_memory(),
            "Used memory should not exceed total memory"
        );

        // CPU checks
        for cpu in sys.cpus() {
            let usage = cpu.cpu_usage();
            assert!(
                usage >= 0.0 && usage <= 95.0,
                "CPU {} usage {:.1}% should be between 0% and 95%",
                cpu.name(),
                usage
            );
        }

        // Process checks
        let current_pid = sysinfo::get_current_pid().expect("Should get current PID");
        if let Some(process) = sys.process(current_pid) {
            assert!(process.memory() > 0, "Server should use some memory");
            assert!(
                process.cpu_usage() >= 0.0,
                "Server CPU usage should be non-negative"
            );
        }

        thread::sleep(Duration::from_millis(100));
    }

    // Cleanup
    server_handle.abort();
    rt.shutdown_timeout(Duration::from_secs(1));
}
