param(
    [string]$CargoToml = "gui/Cargo.toml"
)

$plugins = @(
    'tauri-plugin-clipboard',
    'tauri-plugin-dialog',
    'tauri-plugin-fs',
    'tauri-plugin-http',
    'tauri-plugin-notification',
    'tauri-plugin-os',
    'tauri-plugin-shell',
    'tauri-plugin-upload',
    'tauri-plugin-window-state'
)

Write-Host "Starting plugin enable trial (individual plugin test)..."

$orig = Get-Content $CargoToml -Raw
$results = @{}

foreach ($p in $plugins) {
    Write-Host "\nTrying plugin: $p"

    # Restore original before each attempt
    Set-Content -Path $CargoToml -Value $orig

    # Uncomment the plugin line if present commented
    $lines = Get-Content $CargoToml
    $changed = $false
    $new = @()
    foreach ($line in $lines) {
        if ($line -match "^\s*#\s*($p)\s*=") {
            $new += ($line -replace "^\s*#\s*", "")
            $changed = $true
        } else {
            $new += $line
        }
    }
    if ($changed) { $new | Set-Content $CargoToml -Force }

    Write-Host "Building... (this may take a while)"
    & cargo build -p parsec-cli --features "gui win-preload"
    $build_ok = ($LASTEXITCODE -eq 0)

    if (-not $build_ok) {
        Write-Host "Build failed for plugin $p; recording and continuing.`n"
        $results[$p] = 'build-failed'
        Continue
    }

    Write-Host "Running..."
    & "target\debug\parsec-cli.exe" start
    $rc = $LASTEXITCODE
    Write-Host "Exit code: $rc"

    switch ($rc) {
        0 { $results[$p] = 'ok' }
        3221225785 { $results[$p] = 'entrypoint-not-found' }
        3221225477 { $results[$p] = 'access-violation' }
        3221225786 { $results[$p] = 'dll-init-failed' }
        3221225595 { $results[$p] = 'dll-not-found' }
        default { $results[$p] = "exit-$rc" }
    }

    # revert to original
    Set-Content -Path $CargoToml -Value $orig
}

Write-Host "\nIndividual plugin test results:" -ForegroundColor Cyan
foreach ($k in $results.Keys) {
    Write-Host "$k : $($results[$k])"
}

Write-Host "\nNow attempting to enable all plugins together to test full feature run..." -ForegroundColor Cyan
Set-Content -Path $CargoToml -Value $orig
foreach ($p in $plugins) {
    $orig_lines = Get-Content $CargoToml
    $new = $orig_lines | ForEach-Object { $_ -replace "^\s*#\s*($p)\s*=", '$1=' }
    $new | Set-Content $CargoToml -Force
}

Write-Host "Building with all plugins enabled..."
& cargo build -p parsec-cli --features "gui win-preload"
if ($LASTEXITCODE -ne 0) {
    Write-Host "Build failed when enabling all plugins." -ForegroundColor Yellow
    Set-Content -Path $CargoToml -Value $orig
    Exit 1
}

Write-Host "Running full-feature build..."
& "target\debug\parsec-cli.exe" start
$rc = $LASTEXITCODE
Write-Host "Full-feature run exit code: $rc"

# Restore original
Set-Content -Path $CargoToml -Value $orig

if ($rc -eq 0) {
    Write-Host "Full-feature GUI started successfully." -ForegroundColor Green
    Exit 0
} else {
    Write-Host "Full-feature GUI failed with exit code $rc." -ForegroundColor Red
    Exit 1
}
