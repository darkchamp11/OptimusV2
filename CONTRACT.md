# Execution Contract — Source of Truth

**Status**: ✅ Frozen (Do not change without review)  
**Last Updated**: December 16, 2025

---

## What This Document Defines

This is the **authoritative contract** for Optimus job execution.

Every component (`optimus-api`, `optimus-worker`, `optimus-cli`) **must** adhere to these types.

---

## Core Types

### 1. Language (Strongly Typed)

```rust
pub enum Language {
    Python,
    Java,
    Rust,
}
```

- Serializes to lowercase JSON: `"python"`, `"java"`, `"rust"`
- Will be extended dynamically later
- Currently strict and validated

---

### 2. Job Input (Immutable)

```rust
pub struct JobRequest {
    pub id: Uuid,
    pub language: Language,
    pub source_code: String,
    pub stdin: Option<String>,
    pub timeout_ms: u64,
}
```

**Invariants**:
- Job ID is generated once and never changes
- Source code is write-once
- `timeout_ms` must be validated against `Config::max_timeout_ms`

---

### 3. Job State Machine

```rust
pub enum JobStatus {
    Queued,
    Running,
    Completed,
    Failed,
    TimedOut,
}
```

**State Transitions**:
```
Queued → Running → {Completed | Failed | TimedOut}
```

**Critical Rules**:
- `Queued`: Job pushed to Redis, not yet claimed
- `Running`: Worker has claimed job
- `Completed`: Exit code 0 or successful execution
- `Failed`: Non-zero exit code or execution error
- `TimedOut`: Execution exceeded `timeout_ms`

---

### 4. Execution Output

```rust
pub struct ExecutionResult {
    pub job_id: Uuid,
    pub status: JobStatus,
    pub stdout: String,
    pub stderr: String,
    pub exit_code: Option<i32>,
    pub execution_time_ms: u64,
}
```

**Storage Contract**:
- Written by workers after execution
- Stored at Redis key: `optimus:result:{job_id}`
- Retrieved by API on `GET /job/{id}`

---

## Redis Semantics

### Queue Naming

```rust
pub fn queue_name(language: &Language) -> String {
    format!("optimus:queue:{language}")
}
```

**Examples**:
- Python jobs: `optimus:queue:python`
- Java jobs: `optimus:queue:java`
- Rust jobs: `optimus:queue:rust`

### Result Storage

```rust
pub fn result_key(job_id: &Uuid) -> String {
    format!("optimus:result:{job_id}")
}
```

### Status Tracking

```rust
pub fn status_key(job_id: &Uuid) -> String {
    format!("optimus:status:{job_id}")
}
```

---

## Configuration

```rust
pub struct Config {
    pub redis_url: String,
    pub default_timeout_ms: u64,
    pub max_timeout_ms: u64,
}
```

**Environment Variables**:
- `REDIS_URL` (default: `redis://localhost:6379`)
- `DEFAULT_TIMEOUT_MS` (default: `5000`)
- `MAX_TIMEOUT_MS` (default: `30000`)

---

## What This Enables

✅ API can serialize `JobRequest` and push to Redis  
✅ Worker can deserialize `JobRequest` from Redis  
✅ Worker can serialize `ExecutionResult` and store it  
✅ API can retrieve `ExecutionResult` by job ID  
✅ KEDA can scale based on queue depth per language  
✅ All components share deterministic Redis keys  

---

## What's NOT Defined Yet

❌ Docker image selection  
❌ Sandbox creation logic  
❌ Container stdout/stderr capture  
❌ Retry semantics  
❌ Result TTL policy  

These come later.

---

## Verification

All types have been tested for correct serialization:

```bash
cargo test -p optimus-common
```

**Result**: ✅ 8 tests passed

---

## Next Steps

Now that the contract is frozen:

1. Implement minimal API handler (`POST /submit`)
2. Implement minimal worker loop (`BLPOP → deserialize → log`)
3. Verify end-to-end: submit → queue → dequeue → acknowledge
4. **Do not** touch Docker until this works

---

## Change Policy

Before modifying any type in this contract:

1. Review impact on all dependent components
2. Write migration plan
3. Update tests
4. Commit as breaking change

**Treat this as a database schema.**
