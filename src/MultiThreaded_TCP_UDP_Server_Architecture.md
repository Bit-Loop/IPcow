# IPCow: Multi-Threaded TCP/UDP Server

## Overview

### Features
- **Multi-Threading:** Efficiently handles multiple IPs and ports using system threads.
- **Concurrency:** Asynchronous tasks handle incoming connections and data processing.
- **Dynamic Resource Management:** Adapts to system constraints (e.g., max threads and connections).
- **Error Logging:** Logs failures and ensures graceful recovery.

### Goals
1. **Performance:** Maximize resource utilization while avoiding overload.
2. **Scalability:** Support dynamic IP and port ranges.
3. **Robustness:** Graceful handling of errors and edge cases.
4. **Maintainability:** Clear state transitions and minimal locking.

---

## Final Architecture

### 1. Initialization Phase

**System Resource Discovery:**
```rust
MAX_THREADS = available_parallelism()
MAX_CONN = get_max_conn()
TOTAL_THREADS = MAX_THREADS * user_factor
```

**Address & Port Parsing:**
```rust
(IPS, PORTS) = addr_input()
ADDR_DATA_LIST = [(IP, PORT) for IP in IPS for PORT in PORTS]
```

**Chunk Creation:**
```rust
TOTAL_TASKS = len(ADDR_DATA_LIST)
CHUNK_SIZE = ceil(TOTAL_TASKS / TOTAL_THREADS)
CHUNK_STACK = split_into_chunks(ADDR_DATA_LIST, CHUNK_SIZE)
```

**Shared State Initialization:**
```rust
CONNECTIONS_PROCESSED = AtomicUsize.new(0)
CHUNKS_COMPLETED = AtomicUsize.new(0)
ERROR_LOG = Mutex.new([])
```

---

### 2. Thread Workflow

**Thread Loop:**
```rust
THREAD_WORKER:
    While True:
        Lock CHUNK_STACK
        CHUNK = pop_next_chunk_in_state(CHUNK_STACK, 'Ready')
        Unlock CHUNK_STACK

        If no CHUNK found: Break

        CHUNK.state = 'Processing'

        RESULT = process_chunk(CHUNK)

        If RESULT == SUCCESS:
            CHUNK.state = 'Completed'
            CHUNKS_COMPLETED.increment()
        Else:
            CHUNK.state = 'Error'
            Lock ERROR_LOG
            Append error to ERROR_LOG
            Unlock ERROR_LOG
```

---

### 3. Chunk Processing

**Process Each Chunk:**
```rust
process_chunk(CHUNK):
    For (IP, PORT) in CHUNK:
        Try:
            Bind listener to (IP, PORT)
        Catch ERROR:
            Return ERROR
    Return SUCCESS
```

**Accept Incoming Connections:**
```rust
For each listener in CHUNK:
    While True:
        connection = accept_connection(listener)
        Spawn async task for connection
```

---

### 4. Async Task Workflow

**Handle Incoming Connections:**
```rust
TASK:
    Read data from socket
    If termination sequence detected: Break
    Echo data back to client
    Handle errors gracefully
```

---

### 5. Error Handling

**Chunk-Level Errors:**
```rust
Mark chunks as `Error` and log details.
```

**Connection-Level Errors:**
```rust
Log connection issues without affecting overall progress.
```

**Retry Policy:**
```rust
Optionally retry failed chunks a limited number of times.
```

---

### 6. Finalization

**Monitor Progress:**
```rust
If all CHUNKS_COMPLETED:
    Break main loop
```

**Graceful Shutdown:**
```rust
Close all listeners
Join all threads
Print summary (connections processed, errors logged)
```

---

## Key Improvements

1. **State Machine Integration:**
   - Cleanly manages chunk lifecycles.
2. **Atomic Counters:**
   - Efficient progress tracking without locking.
3. **Minimal Locking:**
   - Locks only chunk stack and error log.
4. **Dynamic Chunk Sizing:**
   - Balances workloads across threads.
5. **Async Connection Handling:**
   - Lightweight tasks ensure scalability.
6. **Comprehensive Logging:**
   - Tracks errors at both chunk and connection levels.

---

## Visualization

### Initial State
```text
Threads: [ T1, T2, T3, T4 ]
Chunks:  [ C1(Idle), C2(Idle), C3(Idle), C4(Idle) ]
```

### After Initialization
```text
Threads: [ T1, T2, T3, T4 ]
Chunks:  [ C1(Ready), C2(Ready), C3(Ready), C4(Ready) ]
```

### During Processing
```text
Threads: [ T1->C1(Processing), T2->C2(Processing), T3->C3(Processing), T4->C4(Processing) ]
Chunks:  [ C1(Processing), C2(Processing), C3(Processing), C4(Processing) ]
```

### Final State
```text
Threads: [ T1(done), T2(done), T3(done), T4(done) ]
Chunks:  [ C1(Completed), C2(Completed), C3(Error), C4(Completed) ]
```
