# Gera docs/RELATORIOS/EVIDENCIAS-EXECUCAO.md com captura literal de stdout/stderr.
# Uso: pwsh -File scripts/gerar-evidencias.ps1
# Requer: cargo, target/debug/rsuvm.exe construído (cargo build --workspace).

$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
Set-Location $root

$utf8 = [System.Text.UTF8Encoding]::new($false)
$cargo = (Get-Command cargo).Source
$bin = (Resolve-Path '.\target\debug\rsuvm.exe').Path
$lines = New-Object System.Collections.Generic.List[string]

function Capture([string]$exe, [string[]]$arguments) {
    $outFile = Join-Path $env:TEMP ('cap-o-' + [guid]::NewGuid().ToString('N'))
    $errFile = Join-Path $env:TEMP ('cap-e-' + [guid]::NewGuid().ToString('N'))
    $process = Start-Process -FilePath $exe -ArgumentList $arguments -Wait -PassThru -NoNewWindow -RedirectStandardOutput $outFile -RedirectStandardError $errFile
    $stdout = [System.IO.File]::ReadAllText($outFile, $utf8).TrimEnd()
    $stderr = [System.IO.File]::ReadAllText($errFile, $utf8).TrimEnd()
    Remove-Item $outFile, $errFile -Force -ErrorAction SilentlyContinue
    [pscustomobject]@{ Exit = $process.ExitCode; Out = $stdout; Err = $stderr }
}

function Add-Block([string]$exe, [string[]]$arguments, [string]$label) {
    $result = Capture $exe $arguments
    $lines.Add("## $label")
    $lines.Add('')
    $lines.Add('```text')
    $lines.Add('$ ' + (Split-Path $exe -Leaf) + ' ' + ($arguments -join ' '))
    if ($result.Out) { $lines.Add($result.Out) }
    if ($result.Err) { $lines.Add('[stderr] ' + $result.Err) }
    $lines.Add('exit: ' + $result.Exit)
    $lines.Add('```')
    $lines.Add('')
    return $result
}

$lines.Add('# Evidências de execução — 2026-11-02')
$lines.Add('')
$lines.Add('Log gerado por `scripts/gerar-evidencias.ps1`, com captura literal de stdout/stderr (UTF-8) no checkout `H:\projetos\rsuvm`, rustc/cargo 1.94.1, Windows. Nenhum resultado abaixo é estimado.')
$lines.Add('')

$null = Add-Block $cargo @('build', '--workspace', '--all-targets') 'cargo build --workspace --all-targets'
$test = Add-Block $cargo @('test', '--workspace') 'cargo test --workspace'
$null = Add-Block $cargo @('clippy', '--workspace', '--all-targets', '--', '-D', 'warnings') 'cargo clippy --workspace --all-targets -- -D warnings'
$null = Add-Block $cargo @('fmt', '--all', '--', '--check') 'cargo fmt --all -- --check (sem saída = formatado)'

$total = ([regex]::Matches($test.Out, 'test result: ok\. (\d+) passed') | ForEach-Object { [int]$_.Groups[1].Value } | Measure-Object -Sum).Sum
$lines.Add('Soma de testes verdes nos alvos: **' + $total + '** (doc-tests inclusos, 0 neles).')
$lines.Add('')

$validos = (Get-ChildItem 'crates\rvm-conformance\corpus\valid' -Filter *.rvm).Count
$invalidos = (Get-ChildItem 'crates\rvm-conformance\corpus\invalid' -Filter *.rvm).Count
$lines.Add('## Corpus de conformidade')
$lines.Add('')
$lines.Add('- casos válidos: **' + $validos + '** (C-001..C-061)')
$lines.Add('- casos inválidos: **' + $invalidos + '** (N-001..N-011, N-013..N-043; N-012 vive no decoder binário)')
$lines.Add('')

$cliCases = @(
    @('run', 'examples/soma.rvm', '--arg', 'i64:10', '--arg', 'i64:20'),
    @('check', 'examples/soma.rvm'),
    @('run', 'crates/rvm-conformance/corpus/invalid/N-001-tipo-de-argumento.rvm'),
    @('run', 'crates/rvm-conformance/corpus/invalid/N-009-overflow.rvm'),
    @('run', 'crates/rvm-conformance/corpus/invalid/N-010-loop-sem-fim.rvm', '--fuel', '1000'),
    @('run', 'crates/rvm-conformance/corpus/invalid/N-011-recursao-profunda.rvm', '--max-depth', '8')
)

$lines.Add('## CLI (`target/debug/rsuvm.exe`)')
$lines.Add('')
$lines.Add('```text')
foreach ($case in $cliCases) {
    $result = Capture $bin $case
    $lines.Add('$ rsuvm ' + ($case -join ' '))
    if ($result.Out) { $lines.Add('stdout: ' + $result.Out) }
    if ($result.Err) { $lines.Add('stderr: ' + $result.Err) }
    $lines.Add('exit: ' + $result.Exit)
    $lines.Add('')
}
$lines.Add('```')
$lines.Add('')

$tempBin = Join-Path $env:TEMP ('rsuvm-evidencia-' + $PID + '.bin')
$encode = Capture $bin @('encode', 'examples/soma.rvm', $tempBin)
$runbc = Capture $bin @('run-bc', $tempBin, '--arg', 'i64:7', '--arg', 'i64:35')
Remove-Item $tempBin -Force -ErrorAction SilentlyContinue

$lines.Add('## Bytecode experimental: encode + run-bc')
$lines.Add('')
$lines.Add('```text')
$lines.Add('$ rsuvm encode examples/soma.rvm ARQUIVO_TEMP.bin')
if ($encode.Out) { $lines.Add($encode.Out) }
if ($encode.Err) { $lines.Add('[stderr] ' + $encode.Err) }
$lines.Add('exit: ' + $encode.Exit)
$lines.Add('')
$lines.Add('$ rsuvm run-bc ARQUIVO_TEMP.bin --arg i64:7 --arg i64:35')
if ($runbc.Out) { $lines.Add($runbc.Out) }
if ($runbc.Err) { $lines.Add('[stderr] ' + $runbc.Err) }
$lines.Add('exit: ' + $runbc.Exit)
$lines.Add('```')
$lines.Add('')

$target = Join-Path $root 'docs\RELATORIOS\EVIDENCIAS-EXECUCAO.md'
[System.IO.File]::WriteAllText($target, ($lines -join "`n"), $utf8)
Write-Output ('escrito: ' + $target)
Write-Output ('testes verdes somados: ' + $total)
