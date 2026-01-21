# j-cmd PowerShell wrapper
# $PROFILE に以下を追加してください:
#   . "C:\path\to\j-init.ps1"
# または、この内容を $PROFILE に直接コピーしてください

function j {
    $prevOutputEncoding = [Console]::OutputEncoding
    $prevInputEncoding = [Console]::InputEncoding
    [Console]::OutputEncoding = [System.Text.Encoding]::UTF8
    [Console]::InputEncoding = [System.Text.Encoding]::UTF8
    try {
        if ($args.Count -eq 0) {
            $result = & j.exe 2>&1
        } else {
            $result = & j.exe @args 2>&1
        }
        
        # 複数行出力（配列）の場合はそのまま表示
        if ($result -is [array]) {
            foreach ($line in $result) {
                Write-Host $line
            }
            return
        }
        
        $output = "$result".Trim()
        if ($output -and (Test-Path -LiteralPath $output -PathType Container -ErrorAction SilentlyContinue)) {
            Set-Location -LiteralPath $output
        } elseif ($output) {
            Write-Host $output -ForegroundColor Yellow
        }
    } finally {
        [Console]::OutputEncoding = $prevOutputEncoding
        [Console]::InputEncoding = $prevInputEncoding
    }
}
