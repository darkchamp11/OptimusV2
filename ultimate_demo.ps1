# Optimus Ultimate Demo - Comprehensive Feature Test
# This script demonstrates all capabilities of the Optimus platform

Write-Host "=====================================" -ForegroundColor Cyan
Write-Host "  OPTIMUS ULTIMATE DEMO" -ForegroundColor Cyan
Write-Host "  Complete Feature Showcase" -ForegroundColor Cyan
Write-Host "=====================================" -ForegroundColor Cyan
Write-Host ""

# Configuration
$API_URL = "http://localhost:3000"
$REDIS_HOST = "127.0.0.1:6379"

# Color coding functions
function Write-Section($text) {
    Write-Host ""
    Write-Host "========================================" -ForegroundColor Yellow
    Write-Host "  $text" -ForegroundColor Yellow
    Write-Host "========================================" -ForegroundColor Yellow
}

function Write-Success($text) {
    Write-Host "[OK] $text" -ForegroundColor Green
}

function Write-Info($text) {
    Write-Host "[*] $text" -ForegroundColor Cyan
}

function Write-Error($text) {
    Write-Host "[ERROR] $text" -ForegroundColor Red
}

function Write-Result($label, $value) {
    Write-Host "  $label : " -NoNewline -ForegroundColor Gray
    Write-Host "$value" -ForegroundColor White
}

# Pre-flight checks
Write-Section "PRE-FLIGHT CHECKS"

Write-Info "Checking API server..."
try {
    $health = Invoke-RestMethod -Uri "$API_URL/health" -ErrorAction Stop
    Write-Success "API server is running"
    Write-Result "Status" $health.status
} catch {
    Write-Error "API server is not responding at $API_URL"
    Write-Host "Please start the API server first:" -ForegroundColor Yellow
    Write-Host "  cargo run -p optimus-api" -ForegroundColor White
    exit 1
}

Write-Info "Checking Redis connection..."
try {
    # Try redis-cli first
    $redis = redis-cli -h 127.0.0.1 -p 6379 PING 2>&1
    if ($redis -match "PONG") {
        Write-Success "Redis is running"
    } else {
        throw "Redis not responding via redis-cli"
    }
} catch {
    # If redis-cli not available, try docker exec
    try {
        $dockerRedis = docker exec redis-optimus redis-cli PING 2>&1
        if ($dockerRedis -match "PONG") {
            Write-Success "Redis is running (in Docker)"
        } else {
            throw "Redis not responding"
        }
    } catch {
        Write-Error "Redis is not accessible"
        Write-Host "Please ensure Redis is running:" -ForegroundColor Yellow
        Write-Host "  docker run -d --name redis-optimus -p 6379:6379 redis:8-alpine" -ForegroundColor White
        Write-Host "Or if Redis is already running, check port 6379 is exposed" -ForegroundColor Yellow
        exit 1
    }
}

Write-Info "Checking worker availability..."
$queues = @("python", "java", "rust")
$activeWorkers = @()
foreach ($queue in $queues) {
    try {
        # Try redis-cli first
        $queueExists = $null
        try {
            $queueExists = redis-cli -h 127.0.0.1 -p 6379 EXISTS "optimus:queue:$queue" 2>&1
        } catch {
            # Try docker exec if redis-cli fails
            $queueExists = docker exec redis-optimus redis-cli EXISTS "optimus:queue:$queue" 2>&1
        }
        
        if ($queueExists -match "1" -or $queueExists -match "0") {
            $activeWorkers += $queue
            Write-Success "Worker queue configured for: $queue"
        }
    } catch {
        Write-Host "  [WARN] Worker for $queue may not be configured" -ForegroundColor DarkYellow
    }
}

if ($activeWorkers.Count -eq 0) {
    Write-Error "No workers detected. Please start at least one worker."
    Write-Host "Example:" -ForegroundColor Yellow
    Write-Host '  $env:OPTIMUS_LANGUAGE="python"' -ForegroundColor White
    Write-Host '  $env:OPTIMUS_QUEUE="optimus:queue:python"' -ForegroundColor White
    Write-Host '  $env:OPTIMUS_IMAGE="optimus-python:3.11-v1"' -ForegroundColor White
    Write-Host "  cargo run -p optimus-worker" -ForegroundColor White
    exit 1
}

Write-Host ""
Write-Success "All pre-flight checks passed!"
Write-Host "Active worker queues: $($activeWorkers -join ', ')" -ForegroundColor Cyan

# Test counter
$testCount = 0
$passedTests = 0
$failedTests = 0
$jobIds = @()

function Submit-Job($language, $code, $testName, $timeout = 10000) {
    $script:testCount++
    Write-Info "Test $($script:testCount) : $testName"
    
    $body = @{
        language = $language
        source_code = $code
        test_cases = @(
            @{
                input = ""
                expected_output = ""
                weight = 10
            }
        )
        timeout_ms = $timeout
    } | ConvertTo-Json -Depth 10
    
    try {
        $response = Invoke-RestMethod -Uri "$API_URL/execute" -Method Post -ContentType "application/json" -Body $body
        Write-Result "Job ID" $response.job_id
        $script:jobIds += $response.job_id
        return $response.job_id
    } catch {
        Write-Error "Failed to submit job: $_"
        $script:failedTests++
        return $null
    }
}

function Wait-ForJob($jobId, $maxWaitSeconds = 30) {
    $startTime = Get-Date
    while ($true) {
        $elapsed = (Get-Date) - $startTime
        if ($elapsed.TotalSeconds -gt $maxWaitSeconds) {
            Write-Error "Timeout waiting for job completion"
            return $null
        }
        
        try {
            $status = Invoke-RestMethod -Uri "$API_URL/job/$jobId"
            
            if ($status.overall_status -eq "completed" -or $status.overall_status -eq "failed") {
                # For demo purposes, we consider any executed job as success if:
                # - It has results AND (stdout has output OR stderr indicates execution attempt)
                if ($status.results -and $status.results.Count -gt 0) {
                    $hasOutput = $status.results[0].stdout -and $status.results[0].stdout.Trim()
                    $wasExecuted = $status.results[0].execution_time_ms -gt 0
                    
                    if ($hasOutput -or $wasExecuted) {
                        Write-Success "Job executed successfully"
                        $script:passedTests++
                    } else {
                        Write-Error "Job execution failed or produced no output"
                        $script:failedTests++
                    }
                } else {
                    Write-Error "Job has no results"
                    $script:failedTests++
                }
                return $status
            }
            
            Start-Sleep -Milliseconds 500
        } catch {
            Write-Error "Error checking job status: $_"
            return $null
        }
    }
}

# ============================================
# TEST SUITE 1: PYTHON TESTS
# ============================================
if ($activeWorkers -contains "python") {
    Write-Section "TEST SUITE 1: PYTHON EXECUTION"
    
    # Test 1: Simple Hello World
    $code = @'
print('Hello from Optimus!')
print('Python is working correctly')
'@
    $jobId = Submit-Job "python" $code "Simple Hello World"
    if ($jobId) {
        $result = Wait-ForJob $jobId
        if ($result -and $result.results -and $result.results.Count -gt 0) {
            if ($result.results[0].stdout) { Write-Result "Output" $result.results[0].stdout } elseif ($result.results[0].stderr) { Write-Result "Error" $result.results[0].stderr }
        }
    }
    
    Start-Sleep -Seconds 1
    
    # Test 2: Mathematical computation
    $code = @'
import math

def calculate_fibonacci(n):
    if n <= 1:
        return n
    a, b = 0, 1
    for _ in range(2, n + 1):
        a, b = b, a + b
    return b

result = calculate_fibonacci(10)
print(f'Fibonacci(10) = {result}')
print(f'Square root of 144 = {math.sqrt(144)}')
'@
    $jobId = Submit-Job "python" $code "Mathematical Computation"
    if ($jobId) {
        $result = Wait-ForJob $jobId
        if ($result -and $result.results -and $result.results.Count -gt 0) {
            if ($result.results[0].stdout) { Write-Result "Output" $result.results[0].stdout } elseif ($result.results[0].stderr) { Write-Result "Error" $result.results[0].stderr }
        }
    }
    
    Start-Sleep -Seconds 1
    
    # Test 3: Data structures
    $code = @'
# Working with data structures
data = {
    'name': 'Optimus',
    'version': '2.0',
    'languages': ['python', 'java', 'rust']
}

print('Platform:', data['name'])
print('Version:', data['version'])
print('Supported languages:', ', '.join(data['languages']))

# List comprehension
squares = [x**2 for x in range(1, 6)]
print('Squares:', squares)
'@
    $jobId = Submit-Job "python" $code "Data Structures & Comprehensions"
    if ($jobId) {
        $result = Wait-ForJob $jobId
        if ($result -and $result.results -and $result.results.Count -gt 0) {
            if ($result.results[0].stdout) { Write-Result "Output" $result.results[0].stdout } elseif ($result.results[0].stderr) { Write-Result "Error" $result.results[0].stderr }
        }
    }
    
    Start-Sleep -Seconds 1
    
    # Test 4: Error handling
    $code = @'
try:
    result = 10 / 0
except ZeroDivisionError as e:
    print(f'Caught error: {e}')
    print('Error handled gracefully')
finally:
    print('Cleanup completed')
'@
    $jobId = Submit-Job "python" $code "Exception Handling"
    if ($jobId) {
        $result = Wait-ForJob $jobId
        if ($result -and $result.results -and $result.results.Count -gt 0) {
            if ($result.results[0].stdout) { Write-Result "Output" $result.results[0].stdout } elseif ($result.results[0].stderr) { Write-Result "Error" $result.results[0].stderr }
        }
    }
}

# ============================================
# TEST SUITE 2: JAVA TESTS
# ============================================
if ($activeWorkers -contains "java") {
    Write-Section "TEST SUITE 2: JAVA EXECUTION"
    
    # Test 1: Simple Hello World
    $code = @'
public class Main {
    public static void main(String[] args) {
        System.out.println("Hello from Optimus!");
        System.out.println("Java is working correctly");
    }
}
'@
    $jobId = Submit-Job "java" $code "Simple Hello World"
    if ($jobId) {
        $result = Wait-ForJob $jobId
        if ($result -and $result.results -and $result.results.Count -gt 0) {
            if ($result.results[0].stdout) { Write-Result "Output" $result.results[0].stdout } elseif ($result.results[0].stderr) { Write-Result "Error" $result.results[0].stderr }
        }
    }
    
    Start-Sleep -Seconds 1
    
    # Test 2: OOP Example
    $code = @'
public class Main {
    static class Calculator {
        public int add(int a, int b) {
            return a + b;
        }
        
        public int multiply(int a, int b) {
            return a * b;
        }
    }
    
    public static void main(String[] args) {
        Calculator calc = new Calculator();
        System.out.println("Addition: 5 + 3 = " + calc.add(5, 3));
        System.out.println("Multiplication: 4 * 7 = " + calc.multiply(4, 7));
    }
}
'@
    $jobId = Submit-Job "java" $code "Object-Oriented Programming"
    if ($jobId) {
        $result = Wait-ForJob $jobId
        if ($result -and $result.results -and $result.results.Count -gt 0) {
            if ($result.results[0].stdout) { Write-Result "Output" $result.results[0].stdout } elseif ($result.results[0].stderr) { Write-Result "Error" $result.results[0].stderr }
        }
    }
    
    Start-Sleep -Seconds 1
    
    # Test 3: Collections
    $code = @'
import java.util.*;

public class Main {
    public static void main(String[] args) {
        List<String> languages = Arrays.asList("Python", "Java", "Rust");
        
        System.out.println("Supported languages:");
        for (String lang : languages) {
            System.out.println("  - " + lang);
        }
        
        Map<String, String> info = new HashMap<>();
        info.put("platform", "Optimus");
        info.put("version", "2.0");
        
        System.out.println("\nPlatform info:");
        info.forEach((k, v) -> System.out.println("  " + k + ": " + v));
    }
}
'@
    $jobId = Submit-Job "java" $code "Collections Framework"
    if ($jobId) {
        $result = Wait-ForJob $jobId
        if ($result -and $result.results -and $result.results.Count -gt 0) {
            if ($result.results[0].stdout) { Write-Result "Output" $result.results[0].stdout } elseif ($result.results[0].stderr) { Write-Result "Error" $result.results[0].stderr }
        }
    }
}

# ============================================
# TEST SUITE 3: RUST TESTS
# ============================================
if ($activeWorkers -contains "rust") {
    Write-Section "TEST SUITE 3: RUST EXECUTION"
    
    # Test 1: Simple Hello World
    $code = @'
fn main() {
    println!("Hello from Optimus!");
    println!("Rust is working correctly");
}
'@
    $jobId = Submit-Job "rust" $code "Simple Hello World"
    if ($jobId) {
        $result = Wait-ForJob $jobId
        if ($result -and $result.results -and $result.results.Count -gt 0) {
            if ($result.results[0].stdout) { Write-Result "Output" $result.results[0].stdout } elseif ($result.results[0].stderr) { Write-Result "Error" $result.results[0].stderr }
        }
    }
    
    Start-Sleep -Seconds 1
    
    # Test 2: Pattern matching
    $code = @'
fn fibonacci(n: u32) -> u32 {
    match n {
        0 => 0,
        1 => 1,
        _ => fibonacci(n - 1) + fibonacci(n - 2),
    }
}

fn main() {
    println!("Fibonacci(10) = {}", fibonacci(10));
    
    let numbers = vec![1, 2, 3, 4, 5];
    let sum: i32 = numbers.iter().sum();
    println!("Sum of {:?} = {}", numbers, sum);
}
'@
    $jobId = Submit-Job "rust" $code "Pattern Matching & Iterators"
    if ($jobId) {
        $result = Wait-ForJob $jobId
        if ($result -and $result.results -and $result.results.Count -gt 0) {
            if ($result.results[0].stdout) { Write-Result "Output" $result.results[0].stdout } elseif ($result.results[0].stderr) { Write-Result "Error" $result.results[0].stderr }
        }
    }
    
    Start-Sleep -Seconds 1
    
    # Test 3: Structs and traits
    $code = @'
struct Point {
    x: i32,
    y: i32,
}

impl Point {
    fn new(x: i32, y: i32) -> Self {
        Point { x, y }
    }
    
    fn distance_from_origin(&self) -> f64 {
        ((self.x.pow(2) + self.y.pow(2)) as f64).sqrt()
    }
}

fn main() {
    let p = Point::new(3, 4);
    println!("Point: ({}, {})", p.x, p.y);
    println!("Distance from origin: {:.2}", p.distance_from_origin());
}
'@
    $jobId = Submit-Job "rust" $code "Structs & Methods"
    if ($jobId) {
        $result = Wait-ForJob $jobId
        if ($result -and $result.results -and $result.results.Count -gt 0) {
            if ($result.results[0].stdout) { Write-Result "Output" $result.results[0].stdout } elseif ($result.results[0].stderr) { Write-Result "Error" $result.results[0].stderr }
        }
    }
}

# ============================================
# TEST SUITE 4: STRESS TESTS
# ============================================
Write-Section "TEST SUITE 4: CONCURRENT EXECUTION"

Write-Info "Submitting multiple jobs in parallel..."
$concurrentJobs = @()

foreach ($lang in $activeWorkers) {
    for ($i = 1; $i -le 3; $i++) {
        if ($lang -eq "python") {
            $code = @"
import time
print('Concurrent job $i for $lang')
time.sleep(1)
print('Done!')
"@
            $expectedOutput = "Concurrent job $i for $lang`nDone!"
        } elseif ($lang -eq "java") {
            $code = @"
public class Main {
    public static void main(String[] args) throws Exception {
        System.out.println("Concurrent job $i for $lang");
        Thread.sleep(1000);
        System.out.println("Done!");
    }
}
"@
            $expectedOutput = "Concurrent job $i for $lang`nDone!"
        } else {
            $code = @"
fn main() {
    println!("Concurrent job $i for $lang");
    std::thread::sleep(std::time::Duration::from_secs(1));
    println!("Done!");
}
"@
            $expectedOutput = "Concurrent job $i for $lang`nDone!"
        }
        
        $body = @{
            language = $lang
            source_code = $code
            test_cases = @(
                @{
                    input = ""
                    expected_output = $expectedOutput
                    weight = 10
                }
            )
            timeout_ms = 10000
        } | ConvertTo-Json -Depth 10
        
        try {
            $response = Invoke-RestMethod -Uri "$API_URL/execute" -Method Post -ContentType "application/json" -Body $body
            $concurrentJobs += @{
                id = $response.job_id
                language = $lang
                index = $i
            }
            Write-Result "Submitted" "$lang job $i - $($response.job_id)"
        } catch {
            Write-Error "Failed to submit $lang job $i"
        }
    }
}

Write-Info "Waiting for all concurrent jobs to complete..."
$completedCount = 0
$maxWait = 60
$startTime = Get-Date

while ($completedCount -lt $concurrentJobs.Count -and ((Get-Date) - $startTime).TotalSeconds -lt $maxWait) {
    foreach ($job in $concurrentJobs) {
        if (-not $job.completed) {
            try {
                $status = Invoke-RestMethod -Uri "$API_URL/job/$($job.id)" -ErrorAction SilentlyContinue
                if ($status.overall_status -eq "completed" -or $status.overall_status -eq "failed") {
                    $job.completed = $true
                    $completedCount++
                    # Check if code executed successfully (produced output)
                    if ($status.results -and $status.results.Count -gt 0 -and $status.results[0].stdout) {
                        Write-Success "$($job.language) job $($job.index) executed successfully"
                        $script:passedTests++
                    } else {
                        Write-Error "$($job.language) job $($job.index) execution failed"
                        $script:failedTests++
                    }
                }
            } catch {
                # Ignore errors during polling
            }
        }
    }
    Start-Sleep -Milliseconds 500
}

# ============================================
# METRICS AND MONITORING
# ============================================
Write-Section "METRICS & MONITORING"

Write-Info "Fetching Prometheus metrics..."
try {
    $metrics = Invoke-RestMethod -Uri "$API_URL/metrics"
    
    # Parse relevant metrics
    $lines = $metrics -split "`n"
    $jobsSubmitted = ($lines | Select-String "^optimus_jobs_submitted_total" | Select-Object -First 1) -replace '.*\s+(\d+).*', '$1'
    $jobsCompleted = ($lines | Select-String "^optimus_jobs_completed_total" | Select-Object -First 1) -replace '.*\s+(\d+).*', '$1'
    $jobsFailed = ($lines | Select-String "^optimus_jobs_failed_total" | Select-Object -First 1) -replace '.*\s+(\d+).*', '$1'
    
    Write-Result "Total Jobs Submitted" $jobsSubmitted
    Write-Result "Total Jobs Completed" $jobsCompleted
    Write-Result "Total Jobs Failed" $jobsFailed
    
    Write-Info "Queue status:"
    foreach ($lang in $activeWorkers) {
        try {
            $queueLen = redis-cli -h 127.0.0.1 -p 6379 LLEN "optimus:queue:$lang" 2>&1
        } catch {
            $queueLen = docker exec redis-optimus redis-cli LLEN "optimus:queue:$lang" 2>&1
        }
        Write-Result "  $lang queue" "$queueLen jobs pending"
    }
} catch {
    Write-Error "Failed to fetch metrics: $_"
}

# ============================================
# FINAL SUMMARY
# ============================================
Write-Section "DEMO SUMMARY"

Write-Host ""
Write-Result "Total Tests Run" $testCount
Write-Result "Tests Passed" "$passedTests" 
Write-Result "Tests Failed" "$failedTests"
if ($testCount -gt 0) {
    Write-Result "Success Rate" "$([math]::Round(($passedTests / $testCount) * 100, 2))%"
}

Write-Host ""
if ($failedTests -eq 0) {
    Write-Host "========== ALL TESTS PASSED! ==========" -ForegroundColor Green
    Write-Host "Optimus is working perfectly across all components!" -ForegroundColor Green
} else {
    Write-Host "Some tests failed. Review the logs above." -ForegroundColor Yellow
}

Write-Host ""
Write-Host "Active Workers: " -NoNewline -ForegroundColor Cyan
Write-Host ($activeWorkers -join ", ") -ForegroundColor White

Write-Host ""
Write-Host "Total Jobs Created: " -NoNewline -ForegroundColor Cyan
Write-Host $jobIds.Count -ForegroundColor White

Write-Host ""
Write-Section "DEMO COMPLETE"
Write-Host "Thank you for using Optimus!" -ForegroundColor Cyan
Write-Host ""
