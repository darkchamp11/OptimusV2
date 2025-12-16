/// Execution Engine - Abstraction for Code Execution
///
/// **Core Responsibility:**
/// Execute source code with test inputs and capture raw outputs.
///
/// **Critical Architectural Boundary:**
/// - Engine knows HOW to execute (Docker, local, sandbox, etc.)
/// - Engine does NOT know scoring rules
/// - Engine does NOT evaluate correctness
/// - Engine returns raw outputs for Evaluator to judge
///
/// **Why This Exists:**
/// Enables swappable execution backends without touching scoring logic.
/// DummyEngine → DockerEngine → K8s Engine → Lambda Engine (all compatible)

use crate::evaluator::TestExecutionOutput;
use optimus_common::types::JobRequest;

/// Execution engine trait
/// 
/// Any implementation must guarantee:
/// 1. Execute source_code with given input
/// 2. Respect timeout_ms
/// 3. Capture stdout/stderr
/// 4. Report timing information
/// 5. Flag timeouts and runtime errors
pub trait ExecutionEngine {
    /// Execute code for a single test case
    ///
    /// ## Arguments
    /// * `source_code` - The source code to execute
    /// * `input` - The stdin input for this test case
    /// * `timeout_ms` - Maximum execution time
    ///
    /// ## Returns
    /// Raw execution output (stdout, stderr, timing, error flags)
    fn execute(
        &self,
        source_code: &str,
        input: &str,
        timeout_ms: u64,
    ) -> TestExecutionOutput;
}

/// Dummy execution engine for testing and validation
///
/// **Dummy Execution Rules:**
/// 1. Treats source_code as ignored
/// 2. stdout = input.trim() (echo semantics)
/// 3. Never times out
/// 4. Never has runtime errors
/// 5. Fixed execution time: 5ms
///
/// **Purpose:**
/// Validate architecture, scoring, and result aggregation
/// before introducing Docker complexity.
pub struct DummyEngine;

impl DummyEngine {
    pub fn new() -> Self {
        DummyEngine
    }
}

impl Default for DummyEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl ExecutionEngine for DummyEngine {
    fn execute(
        &self,
        _source_code: &str,
        input: &str,
        _timeout_ms: u64,
    ) -> TestExecutionOutput {
        // Dummy execution: stdout = input (trimmed)
        let stdout = input.trim().to_string();

        TestExecutionOutput {
            test_id: 0, // Will be set by executor
            stdout,
            stderr: String::new(),
            execution_time_ms: 5, // Fixed dummy time
            timed_out: false,
            runtime_error: false,
        }
    }
}

/// Execute a complete job using the provided execution engine
///
/// This function:
/// 1. Iterates through all test cases
/// 2. Calls engine.execute() for each
/// 3. Collects raw outputs
/// 4. Returns outputs for Evaluator
///
/// ## Arguments
/// * `job` - The job to execute
/// * `engine` - The execution engine to use
///
/// ## Returns
/// Vector of raw execution outputs (one per test case)
pub fn execute_job<E: ExecutionEngine>(
    job: &JobRequest,
    engine: &E,
) -> Vec<TestExecutionOutput> {
    let mut outputs = Vec::new();

    println!("→ Executing {} test cases", job.test_cases.len());
    println!("  Timeout per test: {}ms", job.timeout_ms);
    println!();

    for test_case in &job.test_cases {
        println!("  Executing test {} (id: {})", outputs.len() + 1, test_case.id);

        // Execute with engine
        let mut output = engine.execute(
            &job.source_code,
            &test_case.input,
            job.timeout_ms,
        );

        // Set correct test_id
        output.test_id = test_case.id;

        println!("    Execution time: {}ms", output.execution_time_ms);
        if output.timed_out {
            println!("    ⚠ Timed out");
        }
        if output.runtime_error {
            println!("    ✗ Runtime error");
        }

        outputs.push(output);
    }

    println!();
    println!("→ All test cases executed");

    outputs
}

#[cfg(test)]
mod tests {
    use super::*;
    use optimus_common::types::{Language, TestCase};
    use uuid::Uuid;

    #[test]
    fn test_dummy_engine_echo() {
        let engine = DummyEngine::new();

        let output = engine.execute("ignored code", "hello world", 5000);

        assert_eq!(output.stdout, "hello world");
        assert_eq!(output.stderr, "");
        assert_eq!(output.execution_time_ms, 5);
        assert!(!output.timed_out);
        assert!(!output.runtime_error);
    }

    #[test]
    fn test_dummy_engine_trims_input() {
        let engine = DummyEngine::new();

        let output = engine.execute("code", "  test  \n", 1000);

        assert_eq!(output.stdout, "test");
    }

    #[test]
    fn test_execute_job_multiple_tests() {
        let job = JobRequest {
            id: Uuid::new_v4(),
            language: Language::Python,
            source_code: "# dummy".to_string(),
            test_cases: vec![
                TestCase {
                    id: 1,
                    input: "input1".to_string(),
                    expected_output: "output1".to_string(),
                    weight: 10,
                },
                TestCase {
                    id: 2,
                    input: "input2".to_string(),
                    expected_output: "output2".to_string(),
                    weight: 15,
                },
            ],
            timeout_ms: 5000,
        };

        let engine = DummyEngine::new();
        let outputs = execute_job(&job, &engine);

        assert_eq!(outputs.len(), 2);
        assert_eq!(outputs[0].test_id, 1);
        assert_eq!(outputs[0].stdout, "input1");
        assert_eq!(outputs[1].test_id, 2);
        assert_eq!(outputs[1].stdout, "input2");
    }

    #[test]
    fn test_execute_job_preserves_test_order() {
        let job = JobRequest {
            id: Uuid::new_v4(),
            language: Language::Java,
            source_code: String::new(),
            test_cases: vec![
                TestCase {
                    id: 5,
                    input: "a".to_string(),
                    expected_output: "x".to_string(),
                    weight: 1,
                },
                TestCase {
                    id: 3,
                    input: "b".to_string(),
                    expected_output: "y".to_string(),
                    weight: 1,
                },
                TestCase {
                    id: 7,
                    input: "c".to_string(),
                    expected_output: "z".to_string(),
                    weight: 1,
                },
            ],
            timeout_ms: 1000,
        };

        let engine = DummyEngine::new();
        let outputs = execute_job(&job, &engine);

        assert_eq!(outputs[0].test_id, 5);
        assert_eq!(outputs[1].test_id, 3);
        assert_eq!(outputs[2].test_id, 7);
    }
}
