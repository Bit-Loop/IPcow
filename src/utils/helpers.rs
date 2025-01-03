use std::io::{self, Write};
use std::thread::available_parallelism;
use std::num::NonZeroUsize;

pub fn get_thread_factor() -> usize {
    let system_threads = available_parallelism()
        .unwrap_or(NonZeroUsize::new(1).unwrap())
        .get();
    
    loop {
        print!("System Threads: {}\nEnter thread multiplier: ", system_threads);
        io::stdout().flush().unwrap();

        let mut input = String::new();
        if io::stdin().read_line(&mut input).is_ok() {
            if let Ok(factor) = input.trim().parse::<usize>() {
                if factor > 0 {
                    return system_threads * factor;
                }
            }
        }
        println!("Please enter a positive number");
    }
}

pub fn calculate_optimal_workers(max_workers: usize) -> usize {
    std::cmp::min(max_workers, 100) // Example: cap at 100 workers
}
