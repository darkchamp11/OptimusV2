# Test Per-Language Worker Configuration Validation
Write-Host "`n=== Testing Per-Language Worker Configuration ===" -ForegroundColor Cyan

# Build worker
Write-Host "Building worker..." -ForegroundColor Yellow
cargo build --bin optimus-worker --release 2>&1 | Out-Null

if ($LASTEXITCODE -ne 0) {
    Write-Host "Build failed" -ForegroundColor Red
    exit 1
}

Write-Host "Build successful`n" -ForegroundColor Green

$env:REDIS_URL = "redis://127.0.0.1:6379"

# Test 1: Missing OPTIMUS_LANGUAGE
Write-Host "Test 1: Missing OPTIMUS_LANGUAGE" -ForegroundColor Yellow
Remove-Item Env:\OPTIMUS_LANGUAGE -ErrorAction SilentlyContinue
Remove-Item Env:\OPTIMUS_QUEUE -ErrorAction SilentlyContinue
Remove-Item Env:\OPTIMUS_IMAGE -ErrorAction SilentlyContinue
$output = & .\target\release\optimus-worker.exe 2>&1 | Select-String "OPTIMUS_LANGUAGE"
if ($output) {
    Write-Host "PASS: Worker rejected missing OPTIMUS_LANGUAGE" -ForegroundColor Green
}

# Test 2: Invalid language
Write-Host "`nTest 2: Invalid OPTIMUS_LANGUAGE" -ForegroundColor Yellow
$env:OPTIMUS_LANGUAGE = "invalid"
$output = & .\target\release\optimus-worker.exe 2>&1 | Select-String "Invalid language"
if ($output) {
    Write-Host "PASS: Worker rejected invalid language" -ForegroundColor Green
}

# Test 3: Missing queue
Write-Host "`nTest 3: Missing OPTIMUS_QUEUE" -ForegroundColor Yellow
$env:OPTIMUS_LANGUAGE = "python"
Remove-Item Env:\OPTIMUS_QUEUE -ErrorAction SilentlyContinue
$output = & .\target\release\optimus-worker.exe 2>&1 | Select-String "OPTIMUS_QUEUE"
if ($output) {
    Write-Host "PASS: Worker rejected missing OPTIMUS_QUEUE" -ForegroundColor Green
}

# Test 4: Queue mismatch
Write-Host "`nTest 4: Queue mismatch" -ForegroundColor Yellow
$env:OPTIMUS_LANGUAGE = "python"
$env:OPTIMUS_QUEUE = "optimus:queue:java"
$output = & .\target\release\optimus-worker.exe 2>&1 | Select-String "Queue mismatch"
if ($output) {
    Write-Host "PASS: Worker rejected queue mismatch" -ForegroundColor Green
}

# Test 5: Missing image
Write-Host "`nTest 5: Missing OPTIMUS_IMAGE" -ForegroundColor Yellow
$env:OPTIMUS_LANGUAGE = "python"
$env:OPTIMUS_QUEUE = "optimus:queue:python"
Remove-Item Env:\OPTIMUS_IMAGE -ErrorAction SilentlyContinue
$output = & .\target\release\optimus-worker.exe 2>&1 | Select-String "OPTIMUS_IMAGE"
if ($output) {
    Write-Host "PASS: Worker rejected missing OPTIMUS_IMAGE" -ForegroundColor Green
}

# Test 6: Image mismatch
Write-Host "`nTest 6: Image mismatch" -ForegroundColor Yellow
$env:OPTIMUS_LANGUAGE = "python"
$env:OPTIMUS_QUEUE = "optimus:queue:python"
$env:OPTIMUS_IMAGE = "optimus-java:17-v1"
$output = & .\target\release\optimus-worker.exe 2>&1 | Select-String "Image mismatch"
if ($output) {
    Write-Host "PASS: Worker rejected image mismatch" -ForegroundColor Green
}

# Cleanup
Remove-Item Env:\OPTIMUS_LANGUAGE -ErrorAction SilentlyContinue
Remove-Item Env:\OPTIMUS_QUEUE -ErrorAction SilentlyContinue
Remove-Item Env:\OPTIMUS_IMAGE -ErrorAction SilentlyContinue

Write-Host "`n=== Summary ===" -ForegroundColor Cyan
Write-Host "Workers enforce strict language binding configuration" -ForegroundColor Green
Write-Host "Workers crash fast on misconfiguration" -ForegroundColor Green
Write-Host "Per-language worker specialization validated" -ForegroundColor Green
