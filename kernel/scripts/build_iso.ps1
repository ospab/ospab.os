# Build an ISO with Limine bootloader for the kernel.
# Requirements:
# - xorriso (or genisoimage)
# - limine-install (place in tools/limine)
# - limine.sys placed in tools/limine

$root = Join-Path $PSScriptRoot ".." | Resolve-Path
$project = Split-Path $root -Leaf
$build = Join-Path $root "target\debug\kernel"
# also consider target/x86_64-unknown-none/debug/<binary>
$alternative = Join-Path $root "target\x86_64-unknown-none\debug\ospab-os"

# choose existing build artifact
if (Test-Path $build) {
    $kernelBin = $build
} elseif (Test-Path $alternative) {
    $kernelBin = $alternative
} else {
    # try to find any plausible kernel binary under target
    $found = Get-ChildItem -Path (Join-Path $root 'target') -Recurse -Filter 'ospab-os' -ErrorAction SilentlyContinue | Select-Object -First 1
    if ($found) { $kernelBin = $found.FullName } else { $kernelBin = $null }
}
$isoRoot = Join-Path $root "iso_root"
$bootDir = Join-Path $isoRoot "boot"

if (-not $kernelBin) {
    Write-Host 'Kernel binary not found. Build first: cargo +nightly build' -ForegroundColor Yellow
    exit 1
}

# Prepare directories
Remove-Item -Recurse -Force $isoRoot -ErrorAction SilentlyContinue
New-Item -ItemType Directory -Path $bootDir | Out-Null

# Copy kernel
Copy-Item $kernelBin (Join-Path $bootDir "kernel.elf") -Force

# Copy limine files
$limineFolder = Join-Path $root "tools\limine"
$limineSysPath = Join-Path $limineFolder "limine.sys"
if (-not (Test-Path $limineSysPath)) {
    $alt = Join-Path $limineFolder "limine-bios.sys"
    $alt2 = Join-Path (Join-Path $limineFolder "bin") "limine-bios.sys"
    if (Test-Path $alt) {
        $limineSysPath = $alt
    } elseif (Test-Path $alt2) {
        $limineSysPath = $alt2
    } else {
        Write-Host "Missing limine.sys (or limine-bios.sys) in tools/limine. See tools/limine/README.md" -ForegroundColor Red
        exit 1
    }
}
Copy-Item $limineSysPath $bootDir -Force
Copy-Item (Join-Path $root "limine.cfg") $bootDir -Force

# Create ISO
$isoPath = Join-Path $root "tomatoos.iso"
$xorriso = "xorriso"
& $xorriso -as mkisofs -o $isoPath -b limine.sys -no-emul-boot -boot-load-size 4 -boot-info-table $isoRoot
if ($LASTEXITCODE -ne 0) { Write-Host "xorriso failed" -ForegroundColor Red; exit 1 }

# Run limine-install to install bootloader on the ISO.
# Accept either limine-install.exe (Windows) or limine-install (POSIX build).
$limineInstallExe = Join-Path $limineFolder "limine-install.exe"
$limineInstall = Join-Path $limineFolder "limine-install"
if (Test-Path $limineInstallExe) {
    & $limineInstallExe $isoPath
} elseif (Test-Path $limineInstall) {
    & $limineInstall $isoPath
} else {
    Write-Host "limine-install not found in tools/limine. See tools/limine/README.md for build instructions" -ForegroundColor Red
    exit 1
}
if ($LASTEXITCODE -ne 0) { Write-Host "limine-install failed" -ForegroundColor Red; exit 1 }

# Informational output
Write-Host "ISO created: $isoPath" -ForegroundColor Green
Write-Host "Run in QEMU: qemu-system-x86_64 -cdrom $isoPath -serial stdio -no-reboot"
# Also create a copy named ospab-os.iso
$ospabIso = Join-Path $root "ospab-os.iso"
Copy-Item $isoPath $ospabIso -Force
Write-Host "Also copied ISO to: $ospabIso" -ForegroundColor Green