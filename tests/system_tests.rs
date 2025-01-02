use sysinfo::{System, SystemExt, ProcessorExt};
use std::time::Duration;

#[test]
fn test_system_resources() {
    let mut system = System::new_all();
    system.refresh_all();

    // Test memory usage
    assert!(system.total_memory() > 0, "Total memory should be greater than 0");
    assert!(system.used_memory() <= system.total_memory(), "Used memory should not exceed total memory");
    
    // Test CPU usage
    for processor in system.processors() {
        assert!(processor.cpu_usage() >= 0.0 && processor.cpu_usage() <= 100.0,
            "CPU usage should be between 0% and 100%");
    }
}