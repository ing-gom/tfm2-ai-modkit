# Build one ai-modkit example against the TFM2 Mod SDK.
#   .\build.ps1                 # builds examples\ai_perf
#   .\build.ps1 match_tuner     # builds examples\match_tuner
#   .\build.ps1 draft_ai
#
# SDK location (toolchain pin + mod_api --extern injection come from it):
#   1. $env:TFM2_SDK  if set  — point it at your mod-sdk folder
#   2. otherwise ..\sdk relative to this repo (default workspace layout)
# Override per-invocation with -Sdk <path>. Output: examples\<mod>\<mod>.dll
#
# PREREQUISITE: MSVC linker (VS Build Tools "Desktop C++").
param(
    [string]$Mod = "ai_perf",
    [string]$Sdk = $env:TFM2_SDK
)

$ErrorActionPreference = "Stop"
$root = $PSScriptRoot
$env:Path = "$env:USERPROFILE\.cargo\bin;$env:Path"

# Resolve the SDK: -Sdk / $env:TFM2_SDK first, else the parent workspace's sdk\.
if ([string]::IsNullOrWhiteSpace($Sdk)) {
    $sdk = Join-Path (Split-Path $root -Parent) "sdk"
} else {
    $sdk = $Sdk
}
if (-not (Test-Path (Join-Path $sdk "build_mod.bat"))) {
    throw @"
TFM2 Mod SDK not found at: $sdk
Point the build at your SDK one of these ways:
  - `$env:TFM2_SDK = 'C:\path\to\mod-sdk'`   (persistent for the session)
  - .\build.ps1 $Mod -Sdk 'C:\path\to\mod-sdk'
  - or place this repo beside an `sdk\` folder (default: ..\sdk)
The SDK is the folder containing build_mod.bat, deps\, native\, toolchain_version.txt.
"@
}

# Report the SDK base version and warn on mismatch. The SDK is version-bound, so
# a different base version may change mod_api types/structure — a rebuild (and
# occasionally example tweaks) may be needed. See README "호환성".
$VerifiedSdkVersion = "0.4.7"
$bvFile = Join-Path $sdk "base_version.txt"
$sdkVersion = if (Test-Path $bvFile) { (Get-Content $bvFile -Raw).Trim() } else { "unknown" }
Write-Host "SDK: $sdk (base $sdkVersion)"
if ($sdkVersion -ne "unknown" -and $sdkVersion -ne $VerifiedSdkVersion) {
    Write-Warning "This kit was verified against SDK $VerifiedSdkVersion but you have $sdkVersion. mod_api types/structure may differ — examples may need adjustment."
}

# Pin the exact nightly the SDK was built with — the prebuilt mod_api rlib has a
# version-bound ABI, so a mismatched compiler fails to load in-game. A rustup
# `nightly-YYYY-MM-DD` ships the compiler commit from the PREVIOUS day, so the
# toolchain date = (commit date in toolchain_version.txt) + 1.
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
