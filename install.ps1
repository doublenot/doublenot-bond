$ErrorActionPreference = 'Stop'

$Repo = if ($env:BOND_REPO) { $env:BOND_REPO } else { 'doublenot/doublenot-bond' }
$InstallDir = if ($env:BOND_INSTALL_DIR) { $env:BOND_INSTALL_DIR } else { Join-Path $HOME '.local\bin' }
$Asset = 'doublenot-bond-x86_64-pc-windows-msvc.zip'
$Url = "https://github.com/$Repo/releases/latest/download/$Asset"
$ChecksumsUrl = "https://github.com/$Repo/releases/latest/download/doublenot-bond-checksums.txt"
$TempDir = Join-Path ([System.IO.Path]::GetTempPath()) ("doublenot-bond-" + [System.Guid]::NewGuid().ToString())

New-Item -ItemType Directory -Path $TempDir | Out-Null
New-Item -ItemType Directory -Path $InstallDir -Force | Out-Null

try {
    $ArchivePath = Join-Path $TempDir $Asset
    $ChecksumsPath = Join-Path $TempDir 'doublenot-bond-checksums.txt'
    Invoke-WebRequest -Uri $Url -OutFile $ArchivePath
    Invoke-WebRequest -Uri $ChecksumsUrl -OutFile $ChecksumsPath

    $ExpectedHash = (Select-String -Path $ChecksumsPath -Pattern ([regex]::Escape($Asset) + '$')).Line.Split(' ')[0]
    $ActualHash = (Get-FileHash -Path $ArchivePath -Algorithm SHA256).Hash.ToLowerInvariant()
    if ($ExpectedHash.ToLowerInvariant() -ne $ActualHash) {
        throw 'release archive checksum verification failed'
    }

    Expand-Archive -Path $ArchivePath -DestinationPath $TempDir -Force

    $Binary = Get-ChildItem -Path $TempDir -Recurse -Filter 'doublenot-bond.exe' | Select-Object -First 1
    if (-not $Binary) {
        throw 'doublenot-bond.exe not found in release archive'
    }

    Copy-Item $Binary.FullName (Join-Path $InstallDir 'doublenot-bond.exe') -Force
    Write-Host "installed doublenot-bond to $(Join-Path $InstallDir 'doublenot-bond.exe')"
    if (-not (($env:PATH -split ';') -contains $InstallDir)) {
        Write-Host "add $InstallDir to PATH to run doublenot-bond directly"
    }
}
finally {
    Remove-Item $TempDir -Recurse -Force -ErrorAction SilentlyContinue
}
