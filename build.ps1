# Build one ai-modkit example against the shared SDK in the PARENT workspace.
#   .\build.ps1                 # builds examples\ai_perf
#   .\build.ps1 match_tuner     # builds examples\match_tuner
#   .\build.ps1 draft_ai
#
# Mirrors ../build.ps1 exactly (toolchain pin + mod_api --extern injection) but
# resolves the SDK from the parent workspace and builds from examples\<mod>.
# Output: examples\<mod>\<mod>.dll
#
# PREREQUISITE: MSVC linker (VS Build Tools "Desktop C++"). See ../SETUP.md.
param([string]$Mod = "ai_perf")

$ErrorActionPreference = "Stop"
$root = $PSScriptRoot
$env:Path = "$env:USERPROFILE\.cargo\bin;$env:Path"

# SDK lives one level up, shared with the rest of tfm2-mod-dev.
$sdk = Join-Path (Split-Path $root -Parent) "sdk"
if (-not (Test-Path (Join-Path $sdk "build_mod.bat"))) { throw "SDK not found at $sdk (run ../update-sdk.ps1 first)" }

# Pin the exact nightly the SDK was built with (see ../build.ps1 for the why).
$tcCache = Join-Path $sdk "rustup_toolchain.txt"
if (Test-Path $tcCache) {
    $env:RUSTUP_TOOLCHAIN = (Get-Content $tcCache -Raw).Trim()
} else {
    $tv = Get-Content (Join-Path $sdk "toolchain_version.txt") -Raw
    if ($tv -notmatch '\(([0-9a-f]+)\s+(\d{4}-\d{2}-\d{2})\)') {
        throw "Cannot parse commit date from sdk/toolchain_version.txt: $tv"
    }
    $commitDate = [datetime]::ParseExact($matches[2], 'yyyy-MM-dd', $null)
    $env:RUSTUP_TOOLCHAIN = "nightly-" + $commitDate.AddDays(1).ToString('yyyy-MM-dd')
}

$deps = Join-Path $sdk "deps"
$native = Join-Path $sdk "native"
$modPath = Join-Path $root "examples\$Mod"
if (-not (Test-Path (Join-Path $modPath "Cargo.toml"))) { throw "Cargo.toml not found at $modPath" }

$rlib = (Get-ChildItem "$deps\libmod_api-*.rlib" | Select-Object -First 1).FullName
$sep = [char]31   # CARGO_ENCODED_RUSTFLAGS separator
$flags = @("-L", "dependency=$deps", "--extern", "mod_api=$rlib", "-L", "native=$native")

# Inject prebuilt serde_json only for mods that reference it (keeps its
# serde::Serialize identical to the one mod_api's types implement).
$usesSerdeJson = Select-String -Path "$modPath\src\*.rs" -Pattern "serde_json" -Quiet
if ($usesSerdeJson) {
    $sj = (Get-ChildItem "$deps\libserde_json-*.rlib" | Select-Object -First 1).FullName
    $flags += @("--extern", "serde_json=$sj")
}

$env:CARGO_ENCODED_RUSTFLAGS = ($flags -join $sep)

cargo rustc --release --manifest-path "$modPath\Cargo.toml" --target-dir "$modPath\target" --lib -- --crate-type cdylib
if ($LASTEXITCODE -ne 0) { throw "cargo build failed ($LASTEXITCODE)" }

$built = Join-Path $modPath "target\release\$Mod.dll"
if (-not (Test-Path $built)) { throw "build finished but $built not found" }
$out = Join-Path $modPath "$Mod.dll"
Copy-Item $built $out -Force
Write-Host "Build successful: $out ($((Get-Item $out).Length) bytes)"
