# benchmark_ace.ps1 - O Duelo de Motores (Albedo vs The Giants)
# Script de comparação de performance de parsing HTML

$stressFile = "stress_test.html"
if (-not (Test-Path $stressFile)) {
    Write-Host "Erro: stress_test.html não encontrado! Gere-o primeiro." -ForegroundColor Red
    exit
}

$absolutePath = (Get-Item $stressFile).FullName.Replace("\", "/")
$fileUrl = "file:///$absolutePath"

$fileSizeMB = (Get-Item $stressFile).Length / 1MB
Write-Host "--- Iniciando Benchmark de Parsing (Arquivo: $($fileSizeMB.ToString('F2')) MB) ---" -ForegroundColor Cyan

$browsers = @(
    @{ Name = "Chrome"; Path = "C:\Program Files\Google\Chrome\Application\chrome.exe"; Args = "--headless --dump-dom $fileUrl" },
    @{ Name = "Firefox"; Path = "C:\Program Files\Mozilla Firefox\firefox.exe"; Args = "--headless --dump-dom $fileUrl" },
    @{ Name = "Brave"; Path = "$env:LOCALAPPDATA\BraveSoftware\Brave-Browser\Application\brave.exe"; Args = "--headless --dump-dom $fileUrl" },
    @{ Name = "Brave (Global)"; Path = "C:\Program Files\BraveSoftware\Brave-Browser\Application\brave.exe"; Args = "--headless --dump-dom $fileUrl" },
    @{ Name = "ACE-HTML (v2)"; Path = "target\release\ace_bench.exe"; Args = "" }
)

$results = @()

foreach ($b in $browsers) {
    if (Test-Path $b.Path) {
        Write-Host "Testando $($b.Name)..." -NoNewline
        
        # Medição usando Measure-Command
        try {
            # Warm-up run
            $null = Start-Process -FilePath $b.Path -ArgumentList $b.Args -Wait -WindowStyle Hidden -ErrorAction SilentlyContinue
            
            # Real run
            $time = Measure-Command {
                $p = Start-Process -FilePath $b.Path -ArgumentList $b.Args -Wait -NoNewWindow
            }
            
            $ms = $time.TotalMilliseconds
            $mbps = $fileSizeMB / ($ms / 1000)
            
            Write-Host " [OK] - $($ms.ToString('F2')) ms ($($mbps.ToString('F2')) MB/s)" -ForegroundColor Green
            $results += [PSCustomObject]@{
                Motor = $b.Name
                Tempo_ms = $ms
                Velocidade_MBs = $mbps
            }
        } catch {
            Write-Host " [ERRO]" -ForegroundColor Red
        }
    } else {
        Write-Host "$($b.Name) não encontrado em: $($b.Path)" -ForegroundColor Gray
    }
}

Write-Host "`n--- RESULTADO FINAL ---" -ForegroundColor Yellow
$results | Sort-Object Tempo_ms | Format-Table -AutoSize

if (-not (Test-Path "target\release\ace_bench.exe")) {
    Write-Host "`n[DICA] Para testar o ACE-HTML (Albedo), você precisa compilá-lo primeiro:" -ForegroundColor Cyan
    Write-Host "1. Instale o Visual Studio Build Tools (C++)"
    Write-Host "2. Execute: cargo build --bin ace_bench --release"
}
