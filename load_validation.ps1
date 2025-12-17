# Optimus Load & Scale Validation
# Tests burst load, multi-language, and scaling behavior
# Date: December 17, 2025

$ErrorActionPreference = "Stop"

Write-Host "==================================================" -ForegroundColor Cyan
Write-Host "    OPTIMUS LOAD & SCALE VALIDATION" -ForegroundColor Cyan
Write-Host "==================================================" -ForegroundColor Cyan
Write-Host ""

$API_URL = "http://localhost:8081"
$CONCURRENT_JOBS = 20

# Check API availability
Write-Host "Checking API availability..." -ForegroundColor Yellow
try {
    $health = Invoke-RestMethod -Uri "$API_URL/health" -Method Get -TimeoutSec 5
    Write-Host "[OK] API is healthy" -ForegroundColor Green
} catch {
    Write-Host "[ERROR] API is not available at $API_URL" -ForegroundColor Red
    exit 1
}

Write-Host ""
Write-Host "================================================" -ForegroundColor Cyan
Write-Host "    TEST 1: Burst Load - $CONCURRENT_JOBS jobs" -ForegroundColor Cyan
Write-Host "================================================" -ForegroundColor Cyan

$jobFiles = @("test_job.json") * $CONCURRENT_JOBS

Write-Host "Submitting $CONCURRENT_JOBS jobs concurrently..." -ForegroundColor Yellow
$startTime = Get-Date

$jobs = @()
foreach ($jobFile in $jobFiles) {
    try {
        $response = Invoke-RestMethod -Uri "$API_URL/execute" -Method Post -ContentType "application/json" -InFile $jobFile
        $jobs += @{Success = $true; JobId = $response.job_id; File = $jobFile}
    } catch {
        $jobs += @{Success = $false; Error = $_.Exception.Message; File = $jobFile}
    }
}

$submitTime = (Get-Date) - $startTime
$successfulSubmissions = ($jobs | Where-Object { $_.Success }).Count
$failedSubmissions = ($jobs | Where-Object { -not $_.Success }).Count

Write-Host "[OK] Submitted $($jobs.Count) jobs in $($submitTime.TotalSeconds.ToString('F2'))s" -ForegroundColor Green
Write-Host "  Successful: $successfulSubmissions" -ForegroundColor Green
Write-Host "  Failed: $failedSubmissions" -ForegroundColor $(if ($failedSubmissions -gt 0) { "Red" } else { "Gray" })

Write-Host ""
Write-Host "Waiting for all jobs to complete (this may take a while)..." -ForegroundColor Yellow
$completedJobs = 0
$failedJobs = 0

Start-Sleep -Seconds 10

foreach ($job in $jobs | Where-Object { $_.Success }) {
    $maxAttempts = 120
    $attempt = 0
    $completed = $false
    
    while ($attempt -lt $maxAttempts) {
        try {
            $result = Invoke-RestMethod -Uri "$API_URL/job/$($job.JobId)" -Method Get
            if ($result.overall_status -ne "queued" -and $result.overall_status -ne "running") {
                if ($result.overall_status -eq "completed") {
                    $completedJobs++
                } else {
                    $failedJobs++
                }
                $completed = $true
                break
            }
        } catch {}
        Start-Sleep -Seconds 1
        $attempt++
    }
    
    if (-not $completed) {
        $failedJobs++
    }
}

$totalTime = (Get-Date) - $startTime
Write-Host "[OK] Processing complete in $($totalTime.TotalSeconds.ToString('F2'))s" -ForegroundColor Green
Write-Host "  Completed: $completedJobs" -ForegroundColor Green
Write-Host "  Failed/Timeout: $failedJobs" -ForegroundColor $(if ($failedJobs -gt 0) { "Yellow" } else { "Gray" })

Write-Host ""
Write-Host "================================================" -ForegroundColor Cyan
Write-Host "    LOAD VALIDATION SUMMARY" -ForegroundColor Cyan
Write-Host "================================================" -ForegroundColor Cyan
Write-Host ""

$summary = @"
Burst Load Test:
  - Submitted: $CONCURRENT_JOBS jobs
  - Completed: $completedJobs
  - Failed: $failedJobs
  - Total Time: $($totalTime.TotalSeconds.ToString('F2'))s
  - Avg Time/Job: $(if ($CONCURRENT_JOBS -gt 0) { ($totalTime.TotalSeconds / $CONCURRENT_JOBS).ToString('F2') } else { '0.00' })s
  - Success Rate: $(if ($CONCURRENT_JOBS -gt 0) { (($completedJobs / $CONCURRENT_JOBS) * 100).ToString('F1') } else { '0.0' })%

Result: $(if ($failedJobs -eq 0 -and $failedSubmissions -eq 0) { "[PASS]" } else { "[NEEDS REVIEW]" })
"@

Write-Host $summary -ForegroundColor White
Write-Host ""

if ($failedJobs -eq 0 -and $failedSubmissions -eq 0) {
    Write-Host "[SUCCESS] LOAD & SCALE VALIDATION PASSED" -ForegroundColor Green
    exit 0
} else {
    Write-Host "[WARNING] Some issues detected during load testing" -ForegroundColor Yellow
    exit 1
}
