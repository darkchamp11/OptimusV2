/// Test Evaluator - Language-Agnostic Scoring Logic
///
/// **Core Responsibility:**
/// Compare raw execution outputs against expected outputs and assign scores.
///
/// **Critical Properties:**
/// - Knows nothing about Docker
/// - Knows nothing about language runtimes
/// - Knows nothing about Redis
/// - Pure function: (execution outputs, expected outputs) → scores
///
/// **Scoring Rules:**
/// - Each test case has a weight
/// - score = sum of weights for Passed tests
/// - max_score = sum of all test case weights
/// - overall_status: Completed if any test passed, Failed if all failed
///
/// **Why This Exists:**
/// Separates correctness evaluation from execution mechanism.
/// Guarantees deterministic scoring regardless of execution engine.

use optimus_common::types::{
    ExecutionResult, JobRequest, JobStatus, TestResult, TestStatus,
};

/// Raw execution output for a single test case
/// Produced by ExecutionEngine, consumed by Evaluator
#[derive(Debug, Clone)]
pub struct TestExecutionOutput {
    pub test_id: u32,
    pub stdout: String,
    pub stderr: String,
    pub execution_time_ms: u64,
    pub timed_out: bool,
    pub runtime_error: bool,
}

/// Evaluate all test cases and produce final execution result
///
/// This function:
/// 1. Compares each execution output with expected output
/// 2. Assigns TestStatus based on comparison
/// 3. Calculates score based on weights
/// 4. Determines overall JobStatus
///
/// ## Arguments
/// * `job` - The original job request (for test cases and expected outputs)
/// * `outputs` - Raw execution outputs from the execution engine
///
/// ## Returns
/// Complete ExecutionResult with scores and aggregated status
pub fn evaluate(job: &JobRequest, outputs: Vec<TestExecutionOutput>) -> ExecutionResult {
    let mut test_results = Vec::new();
    let mut total_score = 0u32;
    let max_score: u32 = job.test_cases.iter().map(|tc| tc.weight).sum();

    println!("→ Evaluating {} test outputs", outputs.len());
    println!("  Max possible score: {}", max_score);
    println!();

    for output in outputs {
        // Find corresponding test case
        let test_case = job
            .test_cases
            .iter()
            .find(|tc| tc.id == output.test_id)
            .expect("Test case not found for output");

        // Determine status based on execution output
        let status = if output.runtime_error {
            TestStatus::RuntimeError
        } else if output.timed_out {
            TestStatus::TimeLimitExceeded
        } else {
            // Compare trimmed outputs
            let actual = output.stdout.trim();
            let expected = test_case.expected_output.trim();

            if actual == expected {
                TestStatus::Passed
            } else {
                TestStatus::Failed
            }
        };

        // Update score if passed
        if status == TestStatus::Passed {
            total_score += test_case.weight;
        }

        // Log evaluation (before moving output values)
        println!(
            "  Test {} (id: {}, weight: {}) → {:?}",
            test_results.len() + 1,
            test_case.id,
            test_case.weight,
            status
        );

        if status == TestStatus::Passed {
            println!("    ✓ Output matched");
        } else if status == TestStatus::RuntimeError {
            println!("    ✗ Runtime error");
        } else if status == TestStatus::TimeLimitExceeded {
            println!("    ✗ Timeout");
        } else {
            println!("    ✗ Output mismatch");
            println!("    Expected: \"{}\"", test_case.expected_output.trim());
            println!("    Got:      \"{}\"", output.stdout.trim());
        }

        // Create test result (moves output.stdout and output.stderr)
        let test_result = TestResult {
            test_id: output.test_id,
            status,
            stdout: output.stdout,
            stderr: output.stderr,
            execution_time_ms: output.execution_time_ms,
        };

        test_results.push(test_result);
    }

    // Determine overall status
    let overall_status = if total_score > 0 {
        JobStatus::Completed
    } else {
        JobStatus::Failed
    };

    println!();
    println!("→ Evaluation complete");
    println!("  Score: {} / {}", total_score, max_score);
    println!("  Status: {:?}", overall_status);

    ExecutionResult {
        job_id: job.id,
        overall_status,
        score: total_score,
        max_score,
        results: test_results,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use optimus_common::types::{Language, TestCase};
    use uuid::Uuid;

    #[test]
    fn test_all_pass() {
        let job = JobRequest {
            id: Uuid::new_v4(),
            language: Language::Python,
            source_code: String::new(),
            test_cases: vec![
                TestCase {
                    id: 1,
                    input: "5".to_string(),
                    expected_output: "120".to_string(),
                    weight: 10,
                },
                TestCase {
                    id: 2,
                    input: "3".to_string(),
                    expected_output: "6".to_string(),
                    weight: 15,
                },
            ],
            timeout_ms: 5000,
        };

        let outputs = vec![
            TestExecutionOutput {
                test_id: 1,
                stdout: "120".to_string(),
                stderr: String::new(),
                execution_time_ms: 42,
                timed_out: false,
                runtime_error: false,
            },
            TestExecutionOutput {
                test_id: 2,
                stdout: "6".to_string(),
                stderr: String::new(),
                execution_time_ms: 38,
                timed_out: false,
                runtime_error: false,
            },
        ];

        let result = evaluate(&job, outputs);

        assert_eq!(result.overall_status, JobStatus::Completed);
        assert_eq!(result.score, 25);
        assert_eq!(result.max_score, 25);
        assert_eq!(result.results[0].status, TestStatus::Passed);
        assert_eq!(result.results[1].status, TestStatus::Passed);
    }

    #[test]
    fn test_partial_pass() {
        let job = JobRequest {
            id: Uuid::new_v4(),
            language: Language::Java,
            source_code: String::new(),
            test_cases: vec![
                TestCase {
                    id: 1,
                    input: "input".to_string(),
                    expected_output: "correct".to_string(),
                    weight: 20,
                },
                TestCase {
                    id: 2,
                    input: "input".to_string(),
                    expected_output: "wrong".to_string(),
                    weight: 30,
                },
            ],
            timeout_ms: 5000,
        };

        let outputs = vec![
            TestExecutionOutput {
                test_id: 1,
                stdout: "correct".to_string(),
                stderr: String::new(),
                execution_time_ms: 10,
                timed_out: false,
                runtime_error: false,
            },
            TestExecutionOutput {
                test_id: 2,
                stdout: "incorrect".to_string(),
                stderr: String::new(),
                execution_time_ms: 10,
                timed_out: false,
                runtime_error: false,
            },
        ];

        let result = evaluate(&job, outputs);

        assert_eq!(result.overall_status, JobStatus::Completed);
        assert_eq!(result.score, 20);
        assert_eq!(result.max_score, 50);
        assert_eq!(result.results[0].status, TestStatus::Passed);
        assert_eq!(result.results[1].status, TestStatus::Failed);
    }

    #[test]
    fn test_runtime_error() {
        let job = JobRequest {
            id: Uuid::new_v4(),
            language: Language::Python,
            source_code: String::new(),
            test_cases: vec![TestCase {
                id: 1,
                input: "input".to_string(),
                expected_output: "output".to_string(),
                weight: 10,
            }],
            timeout_ms: 5000,
        };

        let outputs = vec![TestExecutionOutput {
            test_id: 1,
            stdout: String::new(),
            stderr: "RuntimeError: division by zero".to_string(),
            execution_time_ms: 5,
            timed_out: false,
            runtime_error: true,
        }];

        let result = evaluate(&job, outputs);

        assert_eq!(result.overall_status, JobStatus::Failed);
        assert_eq!(result.score, 0);
        assert_eq!(result.results[0].status, TestStatus::RuntimeError);
    }

    #[test]
    fn test_timeout() {
        let job = JobRequest {
            id: Uuid::new_v4(),
            language: Language::Rust,
            source_code: String::new(),
            test_cases: vec![TestCase {
                id: 1,
                input: "input".to_string(),
                expected_output: "output".to_string(),
                weight: 5,
            }],
            timeout_ms: 1000,
        };

        let outputs = vec![TestExecutionOutput {
            test_id: 1,
            stdout: String::new(),
            stderr: String::new(),
            execution_time_ms: 1001,
            timed_out: true,
            runtime_error: false,
        }];

        let result = evaluate(&job, outputs);

        assert_eq!(result.overall_status, JobStatus::Failed);
        assert_eq!(result.score, 0);
        assert_eq!(result.results[0].status, TestStatus::TimeLimitExceeded);
    }

    #[test]
    fn test_whitespace_trimming() {
        let job = JobRequest {
            id: Uuid::new_v4(),
            language: Language::Python,
            source_code: String::new(),
            test_cases: vec![TestCase {
                id: 1,
                input: "input".to_string(),
                expected_output: "hello".to_string(),
                weight: 10,
            }],
            timeout_ms: 5000,
        };

        let outputs = vec![TestExecutionOutput {
            test_id: 1,
            stdout: "  hello  \n".to_string(),
            stderr: String::new(),
            execution_time_ms: 5,
            timed_out: false,
            runtime_error: false,
        }];

        let result = evaluate(&job, outputs);

        assert_eq!(result.overall_status, JobStatus::Completed);
        assert_eq!(result.score, 10);
        assert_eq!(result.results[0].status, TestStatus::Passed);
    }
}
