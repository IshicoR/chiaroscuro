[CmdletBinding()]
param(
    [string]$OutputDirectory = "dist/windows"
)

$ErrorActionPreference = "Stop"

if ($env:OS -ne "Windows_NT") {
    throw "Windows packaging must run on Windows."
}

if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
    throw "cargo is required. Install the Rust toolchain first."
}

if (-not (Get-Command vpk -ErrorAction SilentlyContinue)) {
    throw "vpk is required. Install it with: dotnet tool update -g vpk"
}

$metadata = cargo metadata --no-deps --format-version 1 | ConvertFrom-Json
$package = $metadata.packages | Where-Object { $_.name -eq "chiaro" }
if ($null -eq $package) {
    throw "Could not find the chiaro package in Cargo metadata."
}

$repositoryRoot = (Resolve-Path (Join-Path $PSScriptRoot "../..")).Path
$targetDirectory = Join-Path $metadata.target_directory "release"
$executable = Join-Path $targetDirectory "chiaro.exe"
$outputPath = Join-Path $repositoryRoot $OutputDirectory

cargo build --release -p chiaro
if (-not (Test-Path $executable)) {
    throw "Expected release binary was not produced: $executable"
}

New-Item -ItemType Directory -Force -Path $outputPath | Out-Null
vpk pack `
    -u "com.ishicor.chiaroscuro" `
    -v $package.version `
    -p $targetDirectory `
    -r "win-x64" `
    -e "chiaro.exe" `
    -o $outputPath

Write-Host "Unsigned installer created in $outputPath"
