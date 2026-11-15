param([Parameter(ValueFromRemainingArguments = $true)][string[]]$Files)

$ErrorActionPreference = "Stop"

$certBytes = [Convert]::FromBase64String($env:SIGNING_CERTIFICATE)
$certPath = Join-Path $env:RUNNER_TEMP "signing-cert.pfx"
[IO.File]::WriteAllBytes($certPath, $certBytes)

try {
    $signtool = Get-ChildItem "C:\Program Files (x86)\Windows Kits\10\bin\*\x64\signtool.exe" -Recurse | Sort-Object FullName -Descending | Select-Object -First 1
    if (-not $signtool) { throw "SignTool not found" }

    foreach ($file in $Files) {
        & $signtool.FullName sign /f $certPath /p $env:SIGNING_CERTIFICATE_PASSWORD /fd SHA256 /tr http://timestamp.digicert.com /td SHA256 $file
        if ($LASTEXITCODE -ne 0) { throw "Signing failed for $file" }
    }
}
finally {
    Remove-Item $certPath -Force
}
