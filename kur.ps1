# Kolay'ı derler ve 'kolay' komutunu kullanıcının PATH'ine ekler (Windows).
# Çalıştırmak için PowerShell'de:  powershell -ExecutionPolicy Bypass -File kur.ps1
# Kaldırmak için: Ayarlar > Sistem > Hakkında > Gelişmiş sistem ayarları > Ortam Değişkenleri'nden
# kullanıcı Path'indeki ...\target\release satırını silin.
$ErrorActionPreference = "Stop"
Set-Location $PSScriptRoot
cargo build --release
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
$klasor = Join-Path $PSScriptRoot "target\release"
$yol = [Environment]::GetEnvironmentVariable("Path", "User")
if (-not $yol) { $yol = "" }
if (-not (($yol -split ";") -contains $klasor)) {
    [Environment]::SetEnvironmentVariable("Path", ($yol.TrimEnd(";") + ";" + $klasor).TrimStart(";"), "User")
    Write-Host "PATH'e eklendi: $klasor"
}
Write-Host "Kuruldu: $klasor\kolay.exe"
Write-Host "Yeni bir terminal açıp deneyin: kolay ornekler\01_merhaba.kolay"
