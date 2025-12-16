/// Dummy Test Executor
/// 
/// This module provides a **fake executor** that simulates execution behavior
/// without Docker, language runtimes, or security limits.
/// 
/// ## Purpose
/// Validate test-case semantics, scoring logic, and result aggregation
/// before introducing Docker/Bollard complexity.
/// 
/// ## Dummy Execution Rules
/// For each test case:
/// 1. Treat source_code as echo function: stdout = test_case.input.trim()
/// 2. Compare: stdout == expected_output.trim()
/// 3. Result: Match → Passed, Mismatch → Failed
/// 4. Execution time: Fixed at 5ms per test
/// 
/// ## Scoring Rules
/// - Each test case has a weight
/// - score = sum of weights for Passed tests
/// - max_score = sum of all test weights
/// - overall_status: Completed if any test passed, Failed if all failed

use optimus_common::types::{
    ExecutionResult, JobRequest, JobStatus, TestResult, TestStatus,
};

/// Execute a job using dummy logic (no real execution)
/// 
/// This function validates the complete result pipeline:
/// - Test case iteration
/// - Pass/fail determination
/// - Scoring calculation
/// - Result aggregation
pub fn execute_dummy(job: &JobRequest) -> ExecutionResult {
    let mut test_results = Vec::new();
    let mut total_score = 0u32;
    let max_score: u32 = job.test_cases.iter().map(|tc| tc.weight).sum();
    
    println!("→ Starting dummy execution for job: {}", job.id);
    println!("  Test cases: {}", job.test_cases.len());
    println!("  Max possible score: {}", max_score);
    println!();
    
    // Execute each test case with dummy logic
    for test_case in &job.test_cases {
        // Dummy execution: stdout is just the input (trimmed)
        let stdout = test_case.input.trim().to_string();
        let expected = test_case.expected_output.trim();
        
        // Determine pass/fail
        let status = if stdout == expected {
            TestStatus::Passed
        } else {
            TestStatus::Failed
        };
        
        // Update score if passed
        if status == TestStatus::Passed {
            total_score += test_case.weight;
        }
        
        // Fixed execution time (5ms)
        let execution_time_ms = 5;
        
        // Create test result
        let test_result = TestResult {
            test_id: test_case.id,
            status,
            stdout: stdout.clone(),
            stderr: String::new(),
            execution_time_ms,
        };
        
        // Log individual test result
        println!(
            "  Test {} (id: {}, weight: {}) → {:?}",
            test_results.len() + 1,
            test_case.id,
            test_case.weight,
            status
        );
        
        if status == TestStatus::Passed {
            println!("    ✓ Output matched: \"{}\"", stdout);
        } else {
            println!("    ✗ Expected: \"{}\"", expected);
            println!("    ✗ Got:      \"{}\"", stdout);
        }
        
        test_results.push(test_result);
    }
    
    // Determine overall status
    let overall_status = if total_score > 0 {
        JobStatus::Completed
    } else {
        JobStatus::Failed
    };
    
    println!();
    println!("→ Execution complete");
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
    fn test_all_tests_pass() {
        let job = JobRequest {
            id: Uuid::new_v4(),
            language: Language::Python,
            source_code: "# dummy code".to_string(),
            test_cases: vec![
                TestCase {
                    id: 1,
                    input: "hello".to_string(),
                    expected_output: "hello".to_string(),
                    weight: 10,
                },
                TestCase {
                    id: 2,
                    input: "world".to_string(),
                    expected_output: "world".to_string(),
                    weight: 15,
                },
            ],
            timeout_ms: 5000,
        };
        
        let result = execute_dummy(&job);
        
        assert_eq!(result.overall_status, JobStatus::Completed);
        assert_eq!(result.score, 25); // 10 + 15
        assert_eq!(result.max_score, 25);
        assert_eq!(result.results.len(), 2);
        assert_eq!(result.results[0].status, TestStatus::Passed);
        assert_eq!(result.results[1].status, TestStatus::Passed);
    }
    
    #[test]
    fn test_partial_pass() {
        let job = JobRequest {
            id: Uuid::new_v4(),
            language: Language::Java,
            source_code: "// dummy".to_string(),
            test_cases: vec![
                TestCase {
                    id: 1,
                    input: "42".to_string(),
                    expected_output: "42".to_string(),
                    weight: 20,
                },
                TestCase {
                    id: 2,
                    input: "wrong".to_string(),
                    expected_output: "different".to_string(),
                    weight: 30,
                },
            ],
            timeout_ms: 5000,
        };
        
        let result = execute_dummy(&job);
        
        assert_eq!(result.overall_status, JobStatus::Completed);
        assert_eq!(result.score, 20); // only first test passed
        assert_eq!(result.max_score, 50);
        assert_eq!(result.results[0].status, TestStatus::Passed);
        assert_eq!(result.results[1].status, TestStatus::Failed);
    }
    
    #[test]
    fn test_all_tests_fail() {
        let job = JobRequest {
            id: Uuid::new_v4(),
            language: Language::Rust,
            source_code: "fn main() {}".to_string(),
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
                    weight: 10,
                },
            ],
            timeout_ms: 5000,
        };
        
        let result = execute_dummy(&job);
        
        assert_eq!(result.overall_status, JobStatus::Failed);
        assert_eq!(result.score, 0);
        assert_eq!(result.max_score, 20);
        assert_eq!(result.results[0].status, TestStatus::Failed);
        assert_eq!(result.results[1].status, TestStatus::Failed);
    }
    
    #[test]
    fn test_whitespace_trimming() {
        let job = JobRequest {
            id: Uuid::new_v4(),
            language: Language::Python,
            source_code: "print('test')".to_string(),
            test_cases: vec![
                TestCase {
                    id: 1,
                    input: "  hello  \n".to_string(),
                    expected_output: "hello".to_string(),
                    weight: 5,
                },
            ],
            timeout_ms: 5000,
        };
        
        let result = execute_dummy(&job);
        
        assert_eq!(result.overall_status, JobStatus::Completed);
        assert_eq!(result.score, 5);
        assert_eq!(result.results[0].status, TestStatus::Passed);
    }
    
    #[test]
    fn test_empty_test_cases() {
        let job = JobRequest {
            id: Uuid::new_v4(),
            language: Language::Python,
            source_code: "# no tests".to_string(),
            test_cases: vec![],
            timeout_ms: 5000,
        };
        
        let result = execute_dummy(&job);
        
        assert_eq!(result.overall_status, JobStatus::Failed);
        assert_eq!(result.score, 0);
        assert_eq!(result.max_score, 0);
        assert_eq!(result.results.len(), 0);
    }
    
    #[test]
    fn test_execution_time_is_fixed() {
        let job = JobRequest {
            id: Uuid::new_v4(),
            language: Language::Python,
            source_code: "".to_string(),
            test_cases: vec![
                TestCase {
                    id: 1,
                    input: "test".to_string(),
                    expected_output: "test".to_string(),
                    weight: 1,
                },
            ],
            timeout_ms: 5000,
        };
        
        let result = execute_dummy(&job);
        
        assert_eq!(result.results[0].execution_time_ms, 5);
    }
}
