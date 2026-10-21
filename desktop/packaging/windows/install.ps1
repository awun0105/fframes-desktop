# Installs fframes Studio for the current user from a GitHub release:
#   irm https://github.com/khanhthanhdev/fframes-desktop/releases/latest/download/install.ps1 | iex
# With options:
#   & ([scriptblock]::Create((irm https://github.com/khanhthanhdev/fframes-desktop/releases/latest/download/install.ps1))) -Version v0.1.2 -Quiet
# First offers to install the build prerequisites projects compile with (Visual Studio Build
# Tools with C++ and clang, through winget), then downloads the per-user Setup.exe, checks it
# against the release's SHA256SUMS.txt and runs it. Rust, FFmpeg and the fframes framework come
# with Studio's managed SDK. Studio itself needs no administrator rights; the Build Tools
# installer asks for them. Uninstall from Settings > Apps keeps projects and app data.

# The same discovery as the app's doctor (desktop/crates/studio-sdk/src/doctor.rs).
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

function Install-BuildPrerequisites([string]$Mode, [bool]$Quiet) {
    $msvc = Find-MsvcInstallation
    $libclang = Find-Libclang
    if ($msvc -and $libclang) {
        Write-Host "Build prerequisites found: MSVC in $msvc, libclang in $libclang."
        return
    }
    # The Build Tools command adds Visual Studio's clang as well, which carries libclang.
    $buildTools = @("install", "-e", "--id", "Microsoft.VisualStudio.BuildTools", "--accept-package-agreements", "--accept-source-agreements",
        "--override", "--wait --passive --add Microsoft.VisualStudio.Workload.VCTools --add Microsoft.VisualStudio.Component.VC.Llvm.Clang --includeRecommended")
    $llvm = @("install", "-e", "--id", "LLVM.LLVM", "--accept-package-agreements", "--accept-source-agreements")
    $arguments = if ($msvc) { $llvm } else { $buildTools }
    $missing = if ($msvc) { "LLVM (libclang)" } else { "Visual Studio Build Tools with C++ and clang (a few GB)" }
    $command = "winget " + (($arguments | ForEach-Object { if ($_ -match "\s") { "`"$_`"" } else { $_ } }) -join " ")

    if ($Mode -eq "Skip") {
        Write-Host "Skipping build prerequisites; projects will not compile until $missing is installed: $command"
        return
    }
    if (-not (Get-Command winget -ErrorAction SilentlyContinue)) {
        Write-Warning "Projects need $missing, and winget is not available to install it. Install it from https://visualstudio.microsoft.com/visual-cpp-build-tools/ (Desktop development with C++, plus C++ Clang tools)."
        return
    }
    if ($Mode -eq "Ask") {
        if ($Quiet) {
            Write-Host "Projects need $missing. Install it with: $command (or pass -Prerequisites Install)."
            return
        }
        $answer = Read-Host "Projects need $missing. Install it now with winget? [Y/n]"
        if ($answer -and $answer -notmatch "^(y|yes)$") {
            Write-Host "Not installed. Later, run: $command"
            return
        }
    }
    Write-Host "Installing $missing (Windows asks for administrator rights)"
    & winget @arguments
    if ($LASTEXITCODE -ne 0) {
        # Studio still installs; its setup screen checks the prerequisites again.
        Write-Warning "winget exited with code $LASTEXITCODE. Install $missing yourself with: $command"
    }
}

function Install-FframesStudio {
    [CmdletBinding()]
    param(
        # A release tag such as v0.1.2, or "latest".
        [string]$Version = "latest",
        # No progress window; Setup runs fully silently.
        [switch]$Quiet,
        # Do not start fframes Studio after installing.
        [switch]$NoLaunch,
        # Build prerequisites (MSVC, libclang): Ask before installing them, Install without
        # asking, or Skip. With -Quiet, Ask only prints the command.
        [ValidateSet("Ask", "Install", "Skip")]
        [string]$Prerequisites = "Ask",
        [string]$Repository = "khanhthanhdev/fframes-desktop",
        # Download location override (a mirror, or a local server for testing).
        [string]$BaseUrl
    )
    $ErrorActionPreference = "Stop"
    # Invoke-WebRequest's progress bar slows large downloads down by an order of magnitude.
    $ProgressPreference = "SilentlyContinue"
    $setupName = "fframes-studio-x86_64-pc-windows-msvc-setup.exe"

    if ([Environment]::OSVersion.Version.Major -lt 10) {
        throw "fframes Studio needs Windows 10 or later."
    }
    if (-not [Environment]::Is64BitOperatingSystem) {
        throw "fframes Studio needs 64-bit Windows."
    }
    Install-BuildPrerequisites $Prerequisites $Quiet.IsPresent
    if (-not $BaseUrl) {
        $BaseUrl = if ($Version -eq "latest") {
            "https://github.com/$Repository/releases/latest/download"
        } else {
            "https://github.com/$Repository/releases/download/$Version"
        }
    }
    $BaseUrl = $BaseUrl.TrimEnd("/")
    [Net.ServicePointManager]::SecurityProtocol = [Net.ServicePointManager]::SecurityProtocol -bor [Net.SecurityProtocolType]::Tls12

    $work = Join-Path ([IO.Path]::GetTempPath()) ("fframes-studio-install-" + [guid]::NewGuid().ToString("N"))
    New-Item -ItemType Directory -Path $work | Out-Null
    try {
        Write-Host "Downloading checksums from $BaseUrl"
        $sumsPath = Join-Path $work "SHA256SUMS.txt"
        Invoke-WebRequest -UseBasicParsing -Uri "$BaseUrl/SHA256SUMS.txt" -OutFile $sumsPath
        $expected = $null
        foreach ($line in Get-Content -LiteralPath $sumsPath) {
            $fields = $line.Trim() -split "\s+", 2
            if ($fields.Count -eq 2 -and $fields[1].TrimStart("*") -eq $setupName) {
                $expected = $fields[0].ToUpperInvariant()
            }
        }
        if (-not $expected) {
            throw "The release has no checksum for $setupName."
        }

        Write-Host "Downloading $setupName (about 300 MB)"
        $setupPath = Join-Path $work $setupName
        Invoke-WebRequest -UseBasicParsing -Uri "$BaseUrl/$setupName" -OutFile $setupPath
        $actual = (Get-FileHash -LiteralPath $setupPath -Algorithm SHA256).Hash.ToUpperInvariant()
        if ($actual -ne $expected) {
            throw "Checksum mismatch for $setupName (expected $expected, got $actual); nothing was installed."
        }

        Write-Host "Installing fframes Studio for $env:USERNAME"
        $mode = if ($Quiet) { "/VERYSILENT" } else { "/SILENT" }
        $setup = Start-Process -FilePath $setupPath -ArgumentList $mode, "/SUPPRESSMSGBOXES", "/NORESTART" -Wait -PassThru
        if ($setup.ExitCode -ne 0) {
            throw "Setup failed with exit code $($setup.ExitCode)."
        }
    }
    finally {
        Remove-Item -LiteralPath $work -Recurse -Force -ErrorAction SilentlyContinue
    }

    $key = "HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall\{A4FDB3DF-A64F-4AD1-AA24-245DE70251EC}_is1"
    $location = (Get-ItemProperty -LiteralPath $key -ErrorAction SilentlyContinue).InstallLocation
    if (-not $location) {
        throw "Setup finished but the installation was not registered."
    }
    $executable = Join-Path $location "bin\fframes-studio.exe"
    Write-Host "fframes Studio is installed in $location (Start menu: fframes Studio)."
    Write-Host "On first launch, Setup installs the managed SDK (Rust, FFmpeg and fframes) and checks the build prerequisites again."
    if (-not $NoLaunch) {
        Start-Process -FilePath $executable | Out-Null
    }
}

Install-FframesStudio @args
