# Execution Semantics — Authoritative Contract

**Version**: 1.0  
**Status**: Contract Freeze  
**Date**: December 16, 2025

---

## Overview

This document defines the **authoritative execution semantics** for Optimus job processing. All workers, APIs, and downstream systems must conform to these rules.

**Core Principle**: Test case evaluation is mandatory and deterministic.

---

## Test Case Execution Rules

### 1. Sequential Execution

Test cases **MUST** execute in order by `test_id` (ascending).

- No parallelization within a single job
- Workers iterate `test_cases` sequentially
- Results are collected in execution order

### 2. Per-Test Timeout

The `timeout_ms` field in `JobRequest` applies **per test case**, not per job.

- Each test gets `timeout_ms` to execute
- Exceeding timeout → `TestStatus::TimeLimitExceeded`
- Does not abort remaining tests

### 3. Failure Handling

**Current Behavior (v1.0)**:
- Runtime errors (crashes, exceptions) → `TestStatus::RuntimeError`
- Execution **continues** to next test case
- Job completes even if some tests fail

**Future (configurable)**:
- `stop_on_first_failure: bool` in `JobRequest`
- Allow early termination for CI pipelines

### 4. Output Comparison

Test case evaluation uses **exact string comparison**:

```
actual_output.trim() == expected_output.trim()
```

**Pass Conditions**:
- Output matches exactly (ignoring leading/trailing whitespace)
- Exit code is 0 (runtime success)

**Fail Conditions**:
- Output mismatch → `TestStatus::Failed`
- Non-zero exit code → `TestStatus::RuntimeError`
- Timeout → `TestStatus::TimeLimitExceeded`

---

## Scoring Semantics

### Weighted Scoring

Each `TestCase` has a `weight` field (unsigned integer).

**Calculation**:
```
score = Σ(weight for each passed test)
max_score = Σ(weight for all tests)
```

**Overall Status**:
- `JobStatus::Completed` → all tests passed
- `JobStatus::Failed` → at least one test did not pass
- `JobStatus::TimedOut` → reserved for future job-level timeout

### Partial Credit

Optimus supports **partial success**:

```json
{
  "overall_status": "completed",
  "score": 50,
  "max_score": 100,
  "results": [
    {"test_id": 1, "status": "passed", "weight": 50},
    {"test_id": 2, "status": "failed", "weight": 50}
  ]
}
```

This enables:
- Competitive programming leaderboards
- LMS grading systems
- Progressive debugging feedback

---

## Immutability Guarantees

### Test Cases

`TestCase` is **immutable**:

- Workers receive test cases via `JobRequest`
- Workers **must not** modify `input`, `expected_output`, or `weight`
- Cloning is allowed; mutation is forbidden

### Job Requests

`JobRequest` is **write-once**:

- Created by API
- Enqueued to Redis
- Never modified after creation

---

## Future Considerations

### Features Not Yet Implemented

1. **Job-Level Timeout** — global cap across all tests
2. **Early Termination Flag** — stop on first failure
3. **Custom Comparators** — fuzzy matching, numerical tolerance
4. **Hidden Test Cases** — subset visible to user
5. **Memory Limits** — per-test memory constraints

### Backward Compatibility

When adding features:
- Extend `JobRequest` with **optional** fields
- Default behavior must match current contract
- Versioning via `schema_version: u32` if breaking changes required

---

## Enforcement

### Compile-Time

- Rust type system enforces structure
- Serde guarantees serialization contract
- Tests validate behavior

### Runtime

- Workers log deviations
- API rejects malformed requests
- Redis stores audit trail

### Documentation

- This file is the **source of truth**
- Code comments reference this document
- Changes require PR review + schema version bump

---

## Contract Freeze

As of commit `[to be filled]`, this contract is **frozen**.

No worker or API logic may invent semantics not defined here.

---

**Next Steps**:
1. ✅ Domain types defined (`types.rs`)
2. ⏳ API accepts `JobRequest` with test cases
3. ⏳ Worker executes test cases sequentially
4. ⏳ Dummy evaluator (string comparison)
5. ⏳ Docker sandbox integration
