# Build script for Windows
# Usage: pwsh scripts/build-windows.ps1
$ErrorActionPreference = "Stop"

$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$ProjectDir = Split-Path -Parent $ScriptDir
$BinariesDir = Join-Path $ProjectDir "src-tauri" "binaries"
$TargetTriple = if ($env:TARGET_TRIPLE) { $env:TARGET_TRIPLE } else { "x86_64-pc-windows-msvc" }

New-Item -ItemType Directory -Force -Path $BinariesDir | Out-Null

# Build mock sidecar binaries
foreach ($binary in @("zebrad", "arti", "zaino")) {
    $BinaryPath = Join-Path $BinariesDir "${binary}-${TargetTriple}.exe"
    if (-not (Test-Path $BinaryPath)) {
        Write-Host "Building mock-${binary}..."
        cargo build -p "mock-${binary}" --release --manifest-path (Join-Path $ProjectDir "Cargo.toml")
        if ($LASTEXITCODE -ne 0) { throw "Failed to build mock-${binary}" }
        Copy-Item (Join-Path $ProjectDir "target" "release" "mock-${binary}.exe") $BinaryPath
    } else {
        Write-Host "Using existing ${binary} binary at ${BinaryPath}"
    }
}

# Create a no-op firewall helper stub for Windows
# (Shield Mode is not yet supported on Windows, but Tauri requires the externalBin to exist)
$HelperPath = Join-Path $BinariesDir "zecbox-firewall-helper-${TargetTriple}.exe"
if (-not (Test-Path $HelperPath)) {
    Write-Host "Building firewall-helper stub..."
    # The firewall-helper crate is Unix-only; create a minimal stub
    $StubDir = Join-Path $env:TEMP "zecbox-fw-stub"
    New-Item -ItemType Directory -Force -Path $StubDir | Out-Null
    @"
fn main() {
    eprintln!("Shield Mode firewall helper is not available on Windows.");
    std::process::exit(1);
}
"@ | Set-Content (Join-Path $StubDir "main.rs")
    rustc (Join-Path $StubDir "main.rs") -o $HelperPath
    if ($LASTEXITCODE -ne 0) { throw "Failed to build firewall-helper stub" }
    Remove-Item -Recurse -Force $StubDir
}

# Install frontend dependencies if needed
if (-not (Test-Path (Join-Path $ProjectDir "node_modules"))) {
    Write-Host "Installing frontend dependencies..."
    Push-Location $ProjectDir
    npm ci
    Pop-Location
}

# Build the Tauri app
Write-Host "Building ZecBox for Windows..."
Push-Location $ProjectDir
npx tauri build --target $TargetTriple
Pop-Location

Write-Host ""
Write-Host "Build complete."

$NsisPath = Get-ChildItem -Path (Join-Path $ProjectDir "target" $TargetTriple "release" "bundle" "nsis") -Filter "*.exe" -ErrorAction SilentlyContinue | Select-Object -First 1
if ($NsisPath) {
    Write-Host "NSIS installer: $($NsisPath.FullName)"
    Write-Host "Size: $([math]::Round($NsisPath.Length / 1MB, 2)) MB"
}
