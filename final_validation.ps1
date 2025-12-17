# Optimus Final Validation Suite
# Step 5 - Comprehensive Failure Matrix Validation
# Date: December 17, 2025

$ErrorActionPreference = "Stop"

Write-Host "==================================================" -ForegroundColor Cyan
Write-Host "    OPTIMUS FINAL VALIDATION SUITE" -ForegroundColor Cyan
Write-Host "    Step 5 - Architecture Freeze Validation" -ForegroundColor Cyan
Write-Host "==================================================" -ForegroundColor Cyan
Write-Host ""

# Configuration
$API_URL = "http://localhost:8081"
$global:RESULTS = @()

# Helper Functions
function Submit-Job {
    param($JobFile)
    Write-Host "  Submitting: $JobFile" -ForegroundColor Yellow
    $response = Invoke-RestMethod -Uri "$API_URL/execute" -Method Post -ContentType "application/json" -InFile $JobFile
    return $response.job_id
}

function Get-JobResult {
    param($JobId)
    $maxAttempts = 30
    $attempt = 0
    
    while ($attempt -lt $maxAttempts) {
        try {
            $result = Invoke-RestMethod -Uri "$API_URL/job/$JobId" -Method Get
            if ($result.overall_status -ne "queued" -and $result.overall_status -ne "running") {
                return $result
            }
        } catch {
            # Job not ready yet
        }
        Start-Sleep -Seconds 1
        $attempt++
    }
    throw "Job $JobId timed out waiting for result"
}

function Test-Scenario {
    param(
        [string]$Name,
        [string]$JobFile,
        [string]$ExpectedStatus,
        [scriptblock]$ValidationBlock
    )
    
    Write-Host ""
    Write-Host "[$Name]" -ForegroundColor Cyan
    Write-Host "  Expected: $ExpectedStatus" -ForegroundColor Gray
    
    try {
        $jobId = Submit-Job -JobFile $JobFile
        Write-Host "  Job ID: $jobId" -ForegroundColor Gray
        
        $result = Get-JobResult -JobId $jobId
        Write-Host "  Actual Status: $($result.overall_status)" -ForegroundColor Gray
        
        # Run validation
        $validationResult = & $ValidationBlock $result
        
        if ($validationResult.Success) {
            Write-Host "  [PASS]" -ForegroundColor Green
            $global:RESULTS += [PSCustomObject]@{
                Scenario = $Name
                Status = "PASS"
                Details = $validationResult.Message
            }
        } else {
            Write-Host "  [FAIL]: $($validationResult.Message)" -ForegroundColor Red
            $global:RESULTS += [PSCustomObject]@{
                Scenario = $Name
                Status = "FAIL"
                Details = $validationResult.Message
            }
        }
    } catch {
        Write-Host "  [ERROR]: $_" -ForegroundColor Red
        $global:RESULTS += [PSCustomObject]@{
            Scenario = $Name
            Status = "ERROR"
            Details = $_.Exception.Message
        }
    }
}

# Validation Blocks
$ValidateTimeout = {
    param($result)
    
    $timedOut = $false
    foreach ($test in $result.results) {
        if ($test.status -eq "time_limit_exceeded") {
            $timedOut = $true
            break
        }
    }
    
    if ($timedOut) {
        return @{Success = $true; Message = "Correctly detected timeout"}
    } else {
        return @{Success = $false; Message = "Expected time_limit_exceeded status"}
    }
}

$ValidateOOM = {
    param($result)
    
    $hasRuntimeError = $false
    foreach ($test in $result.results) {
        if ($test.status -eq "runtime_error" -or $test.runtime_error -eq $true) {
            $hasRuntimeError = $true
            break
        }
    }
    
    if ($hasRuntimeError) {
        return @{Success = $true; Message = "OOM correctly caused runtime_error"}
    } else {
        return @{Success = $false; Message = "Expected runtime_error from OOM"}
    }
}

$ValidateSyntaxError = {
    param($result)
    
    if ($result.overall_status -eq "failed") {
        return @{Success = $true; Message = "Syntax error correctly failed"}
    } else {
        return @{Success = $false; Message = "Expected failed status for syntax error"}
    }
}

$ValidateRuntimeError = {
    param($result)
    
    $hasRuntimeError = $false
    foreach ($test in $result.results) {
        if ($test.status -eq "runtime_error" -or $test.runtime_error -eq $true) {
            $hasRuntimeError = $true
            break
        }
    }
    
    if ($hasRuntimeError) {
        return @{Success = $true; Message = "Runtime error correctly detected"}
    } else {
        return @{Success = $false; Message = "Expected runtime_error status"}
    }
}

$ValidatePartialPass = {
    param($result)
    
    $passedCount = 0
    $failedCount = 0
    
    foreach ($test in $result.results) {
        if ($test.status -eq "passed") { $passedCount++ }
        else { $failedCount++ }
    }
    
    if ($passedCount -gt 0 -and $failedCount -gt 0) {
        $score = $result.score
        $maxScore = $result.max_score
        return @{Success = $true; Message = "Partial credit: $score/$maxScore"}
    } else {
        return @{Success = $false; Message = "Expected mixed results, got all pass or all fail"}
    }
}

$ValidateAllFail = {
    param($result)
    
    $allFailed = $true
    foreach ($test in $result.results) {
        if ($test.status -eq "passed") {
            $allFailed = $false
            break
        }
    }
    
    if ($allFailed -and $result.overall_status -eq "failed") {
        return @{Success = $true; Message = "All tests correctly failed"}
    } else {
        return @{Success = $false; Message = "Expected all tests to fail"}
    }
}

$ValidateSuccess = {
    param($result)
    
    if ($result.overall_status -eq "completed" -and $result.score -eq $result.max_score) {
        return @{Success = $true; Message = "Job completed successfully"}
    } else {
        return @{Success = $false; Message = "Expected full success"}
    }
}

# Check API availability
Write-Host "Checking API availability..." -ForegroundColor Yellow
try {
    $health = Invoke-RestMethod -Uri "$API_URL/health" -Method Get -TimeoutSec 5
    Write-Host "[OK] API is healthy" -ForegroundColor Green
} catch {
    Write-Host "[ERROR] API is not available at $API_URL" -ForegroundColor Red
    Write-Host "Please start the API first: `$env:PORT=`"8081`"; cargo run --bin optimus-api" -ForegroundColor Yellow
    exit 1
}

Write-Host ""
Write-Host "================================================" -ForegroundColor Cyan
Write-Host "    FAILURE MATRIX VALIDATION" -ForegroundColor Cyan
Write-Host "================================================" -ForegroundColor Cyan

# Run all validation tests
Test-Scenario -Name "Infinite Loop -> Timeout" `
    -JobFile "test_timeout.json" `
    -ExpectedStatus "time_limit_exceeded" `
    -ValidationBlock $ValidateTimeout

Test-Scenario -Name "Memory Hog -> OOM" `
    -JobFile "test_oom.json" `
    -ExpectedStatus "runtime_error" `
    -ValidationBlock $ValidateOOM

Test-Scenario -Name "Syntax Error -> Failed" `
    -JobFile "test_syntax_error.json" `
    -ExpectedStatus "failed" `
    -ValidationBlock $ValidateSyntaxError

Test-Scenario -Name "Runtime Error -> Failed" `
    -JobFile "test_runtime_error.json" `
    -ExpectedStatus "runtime_error" `
    -ValidationBlock $ValidateRuntimeError

Test-Scenario -Name "Partial Test Pass -> Correct Score" `
    -JobFile "test_job_fail.json" `
    -ExpectedStatus "failed" `
    -ValidationBlock $ValidatePartialPass

Test-Scenario -Name "All Tests Pass -> Completed" `
    -JobFile "test_job.json" `
    -ExpectedStatus "completed" `
    -ValidationBlock $ValidateSuccess

# Summary
Write-Host ""
Write-Host "================================================" -ForegroundColor Cyan
Write-Host "    VALIDATION SUMMARY" -ForegroundColor Cyan
Write-Host "================================================" -ForegroundColor Cyan
Write-Host ""

$passed = ($global:RESULTS | Where-Object { $_.Status -eq "PASS" }).Count
$failed = ($global:RESULTS | Where-Object { $_.Status -eq "FAIL" }).Count
$errors = ($global:RESULTS | Where-Object { $_.Status -eq "ERROR" }).Count
$total = $global:RESULTS.Count

Write-Host "Total Tests: $total" -ForegroundColor White
Write-Host "Passed: $passed" -ForegroundColor Green
Write-Host "Failed: $failed" -ForegroundColor Red
Write-Host "Errors: $errors" -ForegroundColor Yellow
Write-Host ""

$global:RESULTS | Format-Table -AutoSize

if ($failed -eq 0 -and $errors -eq 0) {
    Write-Host ""
    Write-Host "[SUCCESS] ALL VALIDATION TESTS PASSED" -ForegroundColor Green
    Write-Host "Optimus behaves correctly under all failure modes." -ForegroundColor Green
    exit 0
} else {
    Write-Host ""
    Write-Host "[INCOMPLETE] VALIDATION INCOMPLETE" -ForegroundColor Red
    Write-Host "Please fix failing tests before proceeding." -ForegroundColor Yellow
    exit 1
}
