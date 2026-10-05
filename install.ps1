# Installer for the `repository-intelligence` CLI (https://github.com/rustfuture/repository-intelligence).
#
#   irm https://raw.githubusercontent.com/rustfuture/repository-intelligence/main/install.ps1 | iex
#
# Downloads the prebuilt Windows release archive, checks its SHA-256 checksum and
# copies repository-intelligence.exe into an install directory. It needs no administrator rights.
# It does NOT change your PATH: if the install directory is not on it, the script
# prints the command to add it.
#
# Environment variables (all optional):
#   RI_VERSION        Version to install, e.g. 0.3.0 or v0.3.0 (default: latest release)
#   RI_INSTALL_DIR    Where to put repository-intelligence.exe (default: $env:LOCALAPPDATA\repository-intelligence\bin)
#   RI_DOWNLOAD_BASE  Base URL, local directory or file:// URL holding the archive
#                         and its .sha256 file, instead of the GitHub release. Needs
#                         RI_VERSION. Meant for mirrors and testing.
#   RI_REPO           GitHub repository (default: rustfuture/repository-intelligence)
#
# Everything runs inside a function and reports problems with `throw`, never `exit`,
# so piping this script to `iex` cannot close your terminal.

function Install-Tool {
    Set-StrictMode -Version 2
    $ErrorActionPreference = 'Stop'
    $ProgressPreference = 'SilentlyContinue'   # the progress bar makes Windows PowerShell 5.1 downloads very slow

    $repo = if ($env:RI_REPO) { $env:RI_REPO } else { 'rustfuture/repository-intelligence' }
    $fromSource = "cargo install --git https://github.com/$repo repository-intelligence"

    # Windows PowerShell 5.1 does not enable TLS 1.2 by default.
    try { [Net.ServicePointManager]::SecurityProtocol = [Net.ServicePointManager]::SecurityProtocol -bor [Net.SecurityProtocolType]::Tls12 } catch { }

    # --- platform -------------------------------------------------------------
    if ($PSVersionTable.PSVersion.Major -ge 6 -and -not $IsWindows) {
        throw "This installer is for Windows. On macOS or Linux use: curl -fsSL https://raw.githubusercontent.com/$repo/main/install.sh | sh"
    }
    $osArch = [System.Runtime.InteropServices.RuntimeInformation]::OSArchitecture.ToString()
    switch ($osArch) {
        'X64'   { }
        'Arm64' { Write-Host 'Windows on ARM: installing the x64 build, which runs under emulation.' }
        default { throw "Unsupported CPU architecture: $osArch. No prebuilt binary is available; if you have Rust (1.85 or newer), build from source: $fromSource" }
    }
    $target = 'x86_64-pc-windows-msvc'

    # --- version --------------------------------------------------------------
    if ($env:RI_VERSION) {
        $version = $env:RI_VERSION.TrimStart('v')
    } elseif ($env:RI_DOWNLOAD_BASE) {
        throw 'RI_DOWNLOAD_BASE is set, so also set RI_VERSION (for example $env:RI_VERSION = ''0.3.0'').'
    } else {
        Write-Host 'Looking up the latest release...'
        try {
            $release = Invoke-RestMethod -UseBasicParsing -Uri "https://api.github.com/repos/$repo/releases/latest" -Headers @{ 'User-Agent' = 'repository-intelligence-installer' }
        } catch {
            throw 'Could not look up the latest release (no release published yet, no network, or the GitHub API rate limit). Set $env:RI_VERSION, e.g. $env:RI_VERSION = ''0.3.0'''
        }
        $version = ([string]$release.tag_name).TrimStart('v')
    }
    if ($version -notmatch '^[0-9A-Za-z.+-]+$') { throw "Invalid version: $version" }

    $installDir = if ($env:RI_INSTALL_DIR) { $env:RI_INSTALL_DIR } else { Join-Path $env:LOCALAPPDATA 'repository-intelligence\bin' }
    $name = "repository-intelligence-$version-$target"
    $archive = "$name.zip"
    $base = if ($env:RI_DOWNLOAD_BASE) { $env:RI_DOWNLOAD_BASE } else { "https://github.com/$repo/releases/download/v$version" }
    $base = $base.TrimEnd('/', '\')

    $tmp = Join-Path ([IO.Path]::GetTempPath()) ("repository-intelligence-install-" + [Guid]::NewGuid().ToString('N'))
    New-Item -ItemType Directory -Path $tmp | Out-Null
    try {
        Write-Host "Installing repository-intelligence $version for $target"

        # --- download ---------------------------------------------------------
        foreach ($file in @($archive, "$archive.sha256")) {
            $dest = Join-Path $tmp $file
            try {
                if ($base -match '^https?://') {
                    Invoke-WebRequest -UseBasicParsing -Uri "$base/$file" -OutFile $dest
                } else {
                    $localBase = if ($base -match '^file://') { ([Uri]$base).LocalPath } else { $base }
                    Copy-Item -LiteralPath (Join-Path $localBase $file) -Destination $dest
                }
            } catch {
                throw "Could not download $base/$file (is $version a published version for $target?)"
            }
        }

        # --- verify -----------------------------------------------------------
        $sumLine = (Get-Content -LiteralPath (Join-Path $tmp "$archive.sha256") -TotalCount 1)
        $expected = ($sumLine -split '\s+')[0].ToLower()
        if ($expected -notmatch '^[0-9a-f]{64}$') { throw "The checksum file $archive.sha256 is not valid." }
        $actual = (Get-FileHash -LiteralPath (Join-Path $tmp $archive) -Algorithm SHA256).Hash.ToLower()
        if ($expected -ne $actual) {
            Write-Error "Checksum mismatch for $archive`n  expected: $expected`n  actual:   $actual" -ErrorAction Continue
            throw 'Refusing to install a file that does not match its checksum.'
        }
        Write-Host 'Checksum OK'

        # --- unpack and install ----------------------------------------------
        Expand-Archive -LiteralPath (Join-Path $tmp $archive) -DestinationPath $tmp -Force
        $exe = Join-Path $tmp "$name\repository-intelligence.exe"
        if (-not (Test-Path -LiteralPath $exe)) { throw "The archive does not contain $name\repository-intelligence.exe" }

        New-Item -ItemType Directory -Force -Path $installDir | Out-Null
        $installed = Join-Path $installDir 'repository-intelligence.exe'
        try {
            Copy-Item -LiteralPath $exe -Destination $installed -Force
        } catch {
            throw "Could not write $installed. If repository-intelligence is running (for example as an agent hook), close it and try again; or set `$env:RI_INSTALL_DIR to another directory."
        }

        Write-Host "Installed: $installed"
        & $installed --version
        if ($LASTEXITCODE -ne 0) { throw "The installed binary did not run. Build from source instead: $fromSource" }

        # --- PATH hint (never edited automatically) --------------------------
        $dirs = @($env:PATH -split ';' | Where-Object { $_ } | ForEach-Object { $_.TrimEnd('\') })
        if ($dirs -notcontains $installDir.TrimEnd('\')) {
            Write-Host ''
            Write-Host "$installDir is not on your PATH. To add it for your user account, run:"
            Write-Host "  [Environment]::SetEnvironmentVariable('Path', [Environment]::GetEnvironmentVariable('Path','User') + ';$installDir', 'User')"
            Write-Host 'then open a new terminal. (This installer does not change your PATH.)'
        }
        Write-Host ''
        Write-Host "Next: run 'repository-intelligence --help' to get started."
    } finally {
        Remove-Item -LiteralPath $tmp -Recurse -Force -ErrorAction SilentlyContinue
    }
}

Install-Tool
