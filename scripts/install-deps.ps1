# Installs everything an fframes video project needs to build on Windows, in order:
#   1. Visual Studio Build Tools with C++ and clang (the MSVC linker, and libclang for bindgen)
#   2. Rust (rustup, minimal profile: no offline docs)
#   3. FFmpeg 9 shared build (BtbN), checked against its published SHA256
# and sets FFMPEG_DIR, LIBCLANG_PATH and PATH for the current user. Steps already satisfied
# are skipped, so running it again is safe.
#
#   irm https://raw.githubusercontent.com/khanhthanhdev/fframes-desktop/main/scripts/install-deps.ps1 | iex
#
# fframes Studio users do not need this: Studio ships Rust and FFmpeg in its managed SDK and
# its install.ps1 installs the Build Tools.
$ErrorActionPreference = "Stop"
$ProgressPreference = "SilentlyContinue"
[Net.ServicePointManager]::SecurityProtocol = [Net.ServicePointManager]::SecurityProtocol -bor [Net.SecurityProtocolType]::Tls12

$FfmpegBuild = "ffmpeg-n9.0-latest-win64-gpl-shared-9.0"
$FfmpegRelease = "https://github.com/BtbN/FFmpeg-Builds/releases/download/latest"
$FfmpegHome = Join-Path $env:LOCALAPPDATA "fframes\$FfmpegBuild"

function Find-MsvcInstallation {
    $programFiles = if (${env:ProgramFiles(x86)}) { ${env:ProgramFiles(x86)} } else { $env:ProgramFiles }
    $vswhere = Join-Path $programFiles "Microsoft Visual Studio\Installer\vswhere.exe"
    if (-not (Test-Path -LiteralPath $vswhere)) { return $null }
    $path = & $vswhere -latest -products * -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath
    if ($LASTEXITCODE -eq 0 -and $path) { return "$path".Trim() }
    return $null
}

function Find-Libclang {
    $scoop = if ($env:SCOOP) { $env:SCOOP } else { Join-Path $env:USERPROFILE "scoop" }
    $candidates = @($env:LIBCLANG_PATH, (Join-Path $env:ProgramFiles "LLVM\bin"), (Join-Path $scoop "apps\llvm\current\bin"))
    $vs = Find-MsvcInstallation
    if ($vs) { $candidates += Join-Path $vs "VC\Tools\Llvm\x64\bin" }
    foreach ($dir in $candidates) {
        if ($dir -and (Test-Path -LiteralPath (Join-Path $dir "libclang.dll"))) { return $dir }
    }
    return $null
}

function Invoke-Winget([string[]]$Arguments, [string]$What) {
    if (-not (Get-Command winget -ErrorAction SilentlyContinue)) {
        throw "winget is not available to install $What. Install it yourself (https://visualstudio.microsoft.com/visual-cpp-build-tools/: Desktop development with C++, plus C++ Clang tools) and run this again."
    }
    Write-Host "Installing $What (Windows asks for administrator rights)"
    & winget install -e --accept-package-agreements --accept-source-agreements @Arguments
    if ($LASTEXITCODE -ne 0) { throw "winget could not install $What (exit code $LASTEXITCODE)." }
}

function Set-UserVariable([string]$Name, [string]$Value) {
    [Environment]::SetEnvironmentVariable($Name, $Value, "User")
    Set-Item -Path "Env:$Name" -Value $Value
}

function Add-UserPath([string]$Directory) {
    $current = [Environment]::GetEnvironmentVariable("Path", "User")
    $entries = if ($current) { $current -split ";" } else { @() }
    if ($entries -notcontains $Directory) {
        [Environment]::SetEnvironmentVariable("Path", (@($Directory) + $entries) -join ";", "User")
    }
    if (($env:Path -split ";") -notcontains $Directory) { $env:Path = "$Directory;$env:Path" }
}

# 1. MSVC and libclang. Build Tools first: rustup-init would otherwise offer Visual Studio Community.
$msvc = Find-MsvcInstallation
if (-not $msvc) {
    Invoke-Winget @("--id", "Microsoft.VisualStudio.BuildTools", "--override",
        "--wait --passive --add Microsoft.VisualStudio.Workload.VCTools --add Microsoft.VisualStudio.Component.VC.Llvm.Clang --includeRecommended") "Visual Studio Build Tools with C++ and clang (a few GB)"
    $msvc = Find-MsvcInstallation
    if (-not $msvc) { throw "The Build Tools installer finished, but no MSVC x64 tools were found." }
}
Write-Host "MSVC: $msvc"
$libclang = Find-Libclang
if (-not $libclang) {
    Invoke-Winget @("--id", "LLVM.LLVM") "LLVM (libclang)"
    $libclang = Find-Libclang
    if (-not $libclang) { throw "LLVM was installed, but libclang.dll was not found." }
}
Set-UserVariable "LIBCLANG_PATH" $libclang
Write-Host "libclang: $libclang"

# 2. Rust
$cargoBin = Join-Path $env:USERPROFILE ".cargo\bin"
if (-not (Get-Command cargo -ErrorAction SilentlyContinue) -and -not (Test-Path -LiteralPath (Join-Path $cargoBin "cargo.exe"))) {
    Write-Host "Installing Rust (rustup, minimal profile)"
    $rustupInit = Join-Path ([IO.Path]::GetTempPath()) "rustup-init.exe"
    Invoke-WebRequest -UseBasicParsing -Uri "https://static.rust-lang.org/rustup/dist/x86_64-pc-windows-msvc/rustup-init.exe" -OutFile $rustupInit
    & $rustupInit -y --profile minimal --default-toolchain stable
    if ($LASTEXITCODE -ne 0) { throw "rustup-init failed with exit code $LASTEXITCODE." }
    Remove-Item -LiteralPath $rustupInit -Force
}
Add-UserPath $cargoBin
Write-Host "Rust: $(& cargo --version)"

# 3. FFmpeg 9 (fframes links a shared FFmpeg on Windows; proc macros load its DLLs while building)
if (-not (Test-Path -LiteralPath (Join-Path $FfmpegHome "include\libavcodec"))) {
    $work = Join-Path ([IO.Path]::GetTempPath()) ("fframes-ffmpeg-" + [guid]::NewGuid().ToString("N"))
    New-Item -ItemType Directory -Path $work | Out-Null
    try {
        Write-Host "Downloading $FfmpegBuild"
        $zip = Join-Path $work "$FfmpegBuild.zip"
        Invoke-WebRequest -UseBasicParsing -Uri "$FfmpegRelease/$FfmpegBuild.zip" -OutFile $zip
        $sums = Join-Path $work "checksums.sha256"
        Invoke-WebRequest -UseBasicParsing -Uri "$FfmpegRelease/checksums.sha256" -OutFile $sums
        $expected = $null
        foreach ($line in Get-Content -LiteralPath $sums) {
            $fields = $line.Trim() -split "\s+", 2
            if ($fields.Count -eq 2 -and $fields[1].TrimStart("*") -eq "$FfmpegBuild.zip") { $expected = $fields[0] }
        }
        $actual = (Get-FileHash -LiteralPath $zip -Algorithm SHA256).Hash
        if (-not $expected -or $actual -ne $expected.ToUpperInvariant()) {
            throw "Checksum mismatch for $FfmpegBuild.zip (expected $expected, got $actual)."
        }
        Expand-Archive -LiteralPath $zip -DestinationPath $work
        New-Item -ItemType Directory -Force -Path (Split-Path $FfmpegHome) | Out-Null
        if (Test-Path -LiteralPath $FfmpegHome) { Remove-Item -LiteralPath $FfmpegHome -Recurse -Force }
        Move-Item -LiteralPath (Join-Path $work $FfmpegBuild) -Destination $FfmpegHome
    }
    finally {
        Remove-Item -LiteralPath $work -Recurse -Force -ErrorAction SilentlyContinue
    }
}
Set-UserVariable "FFMPEG_DIR" $FfmpegHome
Add-UserPath (Join-Path $FfmpegHome "bin")
Write-Host "FFmpeg: $FfmpegHome"

Write-Host ""
Write-Host "Done. Open a new terminal (FFMPEG_DIR, LIBCLANG_PATH and PATH are set for your user), then:"
Write-Host "  cargo install --locked cargo-fframes"
Write-Host "  cargo fframes new my-video"
