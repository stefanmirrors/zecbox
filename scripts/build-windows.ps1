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

# Build the firewall helper (Shield Mode on Windows uses WinDivert)
$HelperPath = Join-Path $BinariesDir "zecbox-firewall-helper-${TargetTriple}.exe"
if (-not (Test-Path $HelperPath)) {
    Write-Host "Building firewall-helper..."

    # WinDivert SDK must be available. Set WINDIVERT_PATH if not already set.
    if (-not $env:WINDIVERT_PATH) {
        $WinDivertDir = Join-Path $ProjectDir "vendor" "WinDivert"
        if (Test-Path $WinDivertDir) {
            $env:WINDIVERT_PATH = $WinDivertDir
        } else {
            Write-Host "WARNING: WINDIVERT_PATH not set and vendor/WinDivert not found."
            Write-Host "Download WinDivert from https://reqrypt.org/windivert.html"
            Write-Host "and extract to vendor/WinDivert or set WINDIVERT_PATH."
            throw "WinDivert SDK not found"
        }
    }

    cargo build -p firewall-helper --release --manifest-path (Join-Path $ProjectDir "Cargo.toml")
    if ($LASTEXITCODE -ne 0) { throw "Failed to build firewall-helper" }
    Copy-Item (Join-Path $ProjectDir "target" "release" "zecbox-firewall-helper.exe") $HelperPath

    # Copy WinDivert DLL and driver alongside the helper binary
    $WinDivertDll = Join-Path $env:WINDIVERT_PATH "WinDivert.dll"
    $WinDivertSys = Join-Path $env:WINDIVERT_PATH "WinDivert64.sys"
    if (Test-Path $WinDivertDll) {
        Copy-Item $WinDivertDll $BinariesDir
    }
    if (Test-Path $WinDivertSys) {
        Copy-Item $WinDivertSys $BinariesDir
    }
} else {
    Write-Host "Using existing firewall-helper binary at ${HelperPath}"
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
