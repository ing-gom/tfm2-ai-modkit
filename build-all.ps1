# Build every example in examples\. Stops on first failure.
$ErrorActionPreference = "Stop"
$root = $PSScriptRoot
$mods = Get-ChildItem (Join-Path $root "examples") -Directory |
    Where-Object { Test-Path (Join-Path $_.FullName "Cargo.toml") } |
    ForEach-Object { $_.Name }

Write-Host "Building: $($mods -join ', ')"
foreach ($m in $mods) {
    Write-Host "`n=== $m ===" -ForegroundColor Cyan
    & (Join-Path $root "build.ps1") $m
}
Write-Host "`nAll examples built." -ForegroundColor Green
