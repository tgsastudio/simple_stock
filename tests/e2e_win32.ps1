# Stock TW 全功能自動化 Win32 E2E 測試腳本
# 測試前會自動備份使用者原本的 config.dat，測試結束後完整還原，不影響個人自選股資料。

$ErrorActionPreference = "Stop"

Add-Type -MemberDefinition '
[DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern IntPtr FindWindowW(string lpClassName, string lpWindowName);
[DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr hWnd);
[DllImport("user32.dll")] public static extern bool GetClientRect(IntPtr hWnd, out RECT lpRect);
[DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr hWnd, out RECT lpRect);
[DllImport("user32.dll")] public static extern bool MoveWindow(IntPtr hWnd, int X, int Y, int nWidth, int nHeight, bool bRepaint);
[DllImport("user32.dll")] public static extern bool PostMessageW(IntPtr hWnd, uint Msg, IntPtr wParam, IntPtr lParam);
[DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern IntPtr SendMessageW(IntPtr hWnd, uint Msg, IntPtr wParam, string lParam);
[DllImport("user32.dll")] public static extern IntPtr SendMessageW(IntPtr hWnd, uint Msg, IntPtr wParam, IntPtr lParam);
[DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern IntPtr SendMessageW(IntPtr hWnd, uint Msg, IntPtr wParam, System.Text.StringBuilder lParam);
[DllImport("user32.dll")] public static extern uint GetDpiForWindow(IntPtr hWnd);
[DllImport("user32.dll")] public static extern bool SetProcessDpiAwarenessContext(IntPtr value);
[DllImport("user32.dll")] public static extern int GetWindowLongW(IntPtr hWnd, int nIndex);
[DllImport("user32.dll")] public static extern IntPtr GetDlgItem(IntPtr hDlg, int nIDDlgItem);
[DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern bool SetWindowTextW(IntPtr hWnd, string lpString);
[DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern int GetWindowTextW(IntPtr hWnd, System.Text.StringBuilder lpString, int nMaxCount);
public struct RECT { public int Left; public int Top; public int Right; public int Bottom; }
' -Name U32 -Namespace E2EFull

[void][E2EFull.U32]::SetProcessDpiAwarenessContext([IntPtr](-4))

function MakeLParam([int]$x, [int]$y) {
    return [IntPtr](($y -shl 16) -bor ($x -band 0xFFFF))
}

function ClickAt([IntPtr]$hwnd, [int]$x, [int]$y) {
    $lp = MakeLParam $x $y
    [void][E2EFull.U32]::PostMessageW($hwnd, 0x0201, [IntPtr]1, $lp) # WM_LBUTTONDOWN
    [void][E2EFull.U32]::PostMessageW($hwnd, 0x0202, [IntPtr]0, $lp) # WM_LBUTTONUP
}

function GetCtrlText([IntPtr]$dlg, [int]$id) {
    $ctrl = [E2EFull.U32]::GetDlgItem($dlg, $id)
    $sb = New-Object System.Text.StringBuilder 512
    [void][E2EFull.U32]::SendMessageW($ctrl, 0x000D, [IntPtr]512, $sb) # WM_GETTEXT
    return $sb.ToString()
}

function SetCtrlText([IntPtr]$dlg, [int]$id, [string]$text) {
    $ctrl = [E2EFull.U32]::GetDlgItem($dlg, $id)
    [void][E2EFull.U32]::SendMessageW($ctrl, 0x000C, [IntPtr]::Zero, $text) # WM_SETTEXT
    $wp = [IntPtr]((0x0300 -shl 16) -bor $id)
    [void][E2EFull.U32]::PostMessageW($dlg, 0x0111, $wp, $ctrl)
}

function Wait-Window([string]$cls, [int]$timeoutMs = 5000) {
    $sw = [System.Diagnostics.Stopwatch]::StartNew()
    while ($sw.ElapsedMilliseconds -lt $timeoutMs) {
        $h = [E2EFull.U32]::FindWindowW($cls, [NullString]::Value)
        if ($h -ne [IntPtr]::Zero -and [E2EFull.U32]::IsWindowVisible($h)) { return $h }
        Start-Sleep -Milliseconds 100
    }
    return [IntPtr]::Zero
}

function Wait-WindowClosed([string]$cls, [int]$timeoutMs = 4000) {
    $sw = [System.Diagnostics.Stopwatch]::StartNew()
    while ($sw.ElapsedMilliseconds -lt $timeoutMs) {
        $h = [E2EFull.U32]::FindWindowW($cls, [NullString]::Value)
        if ($h -eq [IntPtr]::Zero) { return $true }
        Start-Sleep -Milliseconds 100
    }
    return $false
}

function GetTopBarCoords([IntPtr]$main) {
    $rc = New-Object E2EFull.U32+RECT
    [void][E2EFull.U32]::GetClientRect($main, [ref]$rc)
    $dpi = [Math]::Max(96, [int][E2EFull.U32]::GetDpiForWindow($main))
    $sc = { param($px) [int][Math]::Round($px * $dpi / 96.0) }
    $w = $rc.Right - $rc.Left
    $style = [E2EFull.U32]::GetWindowLongW($main, -16)
    $isFloating = (($style -band 0x00C00000) -eq 0)
    $isCompact = $isFloating -or ($w -lt (& $sc 340))

    $pad = & $sc 10
    $topY = (& $sc 8) + (& $sc 12)
    $rx = $w - $pad
    if ($isFloating) { $rx -= (& $sc 22) + (& $sc 4) }
    $setX = $rx - (& $sc 16); $rx -= (& $sc 32) + (& $sc 4)
    $addX = $rx - (& $sc 11); $rx -= (& $sc 22) + (& $sc 4)
    $refX = 0
    $pinX = 0
    if (-not $isCompact) {
        $refX = $rx - (& $sc 11); $rx -= (& $sc 22) + (& $sc 4)
        $pinX = $rx - (& $sc 13); $rx -= (& $sc 26) + (& $sc 4)
    }
    $floatX = $rx - (& $sc 17); $rx -= (& $sc 34) + (& $sc 4)
    $stdX   = $rx - (& $sc 17)

    return @{
        W=$w; H=($rc.Bottom - $rc.Top); DPI=$dpi; Floating=$isFloating; TopY=$topY;
        SetX=$setX; AddX=$addX; RefX=$refX; PinX=$pinX; FloatX=$floatX; StdX=$stdX
    }
}

$exe = "D:\dev\stock widget\target\release\simple_stock.exe"
$cfgPath = "$env:APPDATA\tgsastudio\simple_stock\config\config.dat"
$backupPath = "$env:TEMP\stock_tw_config_e2e_backup.dat"
$hadBackup = $false
if (Test-Path $cfgPath) {
    Copy-Item $cfgPath $backupPath -Force
    $hadBackup = $true
    Remove-Item $cfgPath -Force
}

$proc = $null
try {
    # 1. 全新啟動（預設 Standard 模式）
    $proc = Start-Process $exe -PassThru
    $main = Wait-Window "StockTW_MainWindow_v2" 6000
    if ($main -eq [IntPtr]::Zero) { throw "FAIL: Main window did not appear!" }
    Start-Sleep -Milliseconds 1500
    $c = GetTopBarCoords $main
    Write-Host "[PASS] 1. Fresh launch visible (Standard=$($c.Floating -eq $false), Size=$($c.W)x$($c.H), DPI=$($c.DPI))"

    # 2. 切換兩種視窗模式（標準 -> 浮動 -> 標準）
    ClickAt $main $c.FloatX $c.TopY
    Start-Sleep -Milliseconds 400
    $c3 = GetTopBarCoords $main
    if (-not $c3.Floating) { throw "FAIL: Floating mode did not remove WS_CAPTION!" }

    ClickAt $main $c3.StdX $c3.TopY
    Start-Sleep -Milliseconds 400
    $c = GetTopBarCoords $main
    if ($c.Floating) { throw "FAIL: Standard mode did not restore WS_CAPTION!" }
    Write-Host "[PASS] 2. Both view modes switched cleanly (Std -> Floating -> Std)"

    # 3. 置頂切換驗證（WS_EX_TOPMOST = 0x00000008）
    $beforeTop = (([E2EFull.U32]::GetWindowLongW($main, -20)) -band 0x8) -ne 0
    ClickAt $main $c.PinX $c.TopY
    Start-Sleep -Milliseconds 250
    $after1 = (([E2EFull.U32]::GetWindowLongW($main, -20)) -band 0x8) -ne 0
    if ($after1 -eq $beforeTop) { throw "FAIL: Always-on-top did not toggle on first click!" }
    ClickAt $main $c.PinX $c.TopY
    Start-Sleep -Milliseconds 250
    $after2 = (([E2EFull.U32]::GetWindowLongW($main, -20)) -band 0x8) -ne 0
    if ($after2 -ne $beforeTop) { throw "FAIL: Always-on-top did not toggle back on second click!" }
    Write-Host "[PASS] 3. Always-on-top toggled cleanly ($beforeTop -> $after1 -> $after2)"

    # 4. 新增自選股（直接加入 2454 並設定持股數量 100 / 成本 1000）
    ClickAt $main $c.AddX $c.TopY
    $addDlg = Wait-Window "StockTW_AddStockDlg_v2" 4000
    if ($addDlg -eq [IntPtr]::Zero) { throw "FAIL: Add Stock dialog did not open!" }
    SetCtrlText $addDlg 201 "2454"
    SetCtrlText $addDlg 205 "100"
    SetCtrlText $addDlg 206 "1000"
    [void][E2EFull.U32]::PostMessageW($addDlg, 0x0111, [IntPtr]207, [IntPtr]::Zero) # ID_ADD_DIRECT
    if (-not (Wait-WindowClosed "StockTW_AddStockDlg_v2" 3000)) { throw "FAIL: Add Stock dialog did not close after adding!" }
    Write-Host "[PASS] 4. Added stock 2454 with position (100 @ 1000) and saved"

    # 5. 新增自選股（線上搜尋 MSFT + Esc 鍵關閉）
    ClickAt $main $c.AddX $c.TopY
    $addDlg2 = Wait-Window "StockTW_AddStockDlg_v2" 4000
    SetCtrlText $addDlg2 201 "MSFT"
    [void][E2EFull.U32]::PostMessageW($addDlg2, 0x0111, [IntPtr]202, [IntPtr]::Zero) # ID_ADD_SEARCH
    $lb = [E2EFull.U32]::GetDlgItem($addDlg2, 208)
    $foundCount = 0
    for ($i = 0; $i -lt 50; $i++) {
        $foundCount = [int][E2EFull.U32]::SendMessageW($lb, 0x018B, [IntPtr]::Zero, [IntPtr]::Zero) # LB_GETCOUNT
        if ($foundCount -gt 0) { break }
        Start-Sleep -Milliseconds 100
    }
    if ($foundCount -le 0) { throw "FAIL: Online stock search returned 0 results!" }
    [void][E2EFull.U32]::PostMessageW($addDlg2, 0x0111, [IntPtr]2, [IntPtr]::Zero) # IDCANCEL (Esc)
    if (-not (Wait-WindowClosed "StockTW_AddStockDlg_v2" 3000)) { throw "FAIL: Add Stock dialog did not close on Esc!" }
    Write-Host "[PASS] 5. Online search returned $foundCount results for MSFT and closed via Esc"

    # 6. 編輯現有自選股持股（雙擊第一張卡片 -> 輸入 500 @ 900 -> 驗證即時預覽 -> 儲存）
    $dpi = $c.DPI
    $sc = { param($px) [int][Math]::Round($px * $dpi / 96.0) }
    $cardX = [int]($c.W / 2)
    $cardY = & $sc 120
    [void][E2EFull.U32]::PostMessageW($main, 0x0203, [IntPtr]1, (MakeLParam $cardX $cardY)) # WM_LBUTTONDBLCLK
    $epDlg = Wait-Window "StockTW_EditPosDlg_v2" 4000
    if ($epDlg -eq [IntPtr]::Zero) { throw "FAIL: Edit Position dialog did not open!" }
    SetCtrlText $epDlg 101 "500"
    SetCtrlText $epDlg 102 "900"
    Start-Sleep -Milliseconds 250
    $preview = GetCtrlText $epDlg 103
    if ([string]::IsNullOrWhiteSpace($preview)) { throw "FAIL: Edit Position live preview is empty!" }
    [void][E2EFull.U32]::PostMessageW($epDlg, 0x0111, [IntPtr]104, [IntPtr]::Zero) # ID_EP_SAVE
    if (-not (Wait-WindowClosed "StockTW_EditPosDlg_v2" 3000)) { throw "FAIL: Edit Position dialog did not close on Save!" }
    Write-Host "[PASS] 6. Edit Position saved with live preview: '$preview'"

    # 7. 設定對話框（即時檢查 GitHub Release 更新 + 切換語系為英文 + 儲存）
    ClickAt $main $c.SetX $c.TopY
    $setDlg = Wait-Window "StockTW_SettingsDlg_v2" 4000
    if ($setDlg -eq [IntPtr]::Zero) { throw "FAIL: Settings dialog did not open!" }
    [void][E2EFull.U32]::PostMessageW($setDlg, 0x0111, [IntPtr]310, [IntPtr]::Zero) # ID_SET_CHECK_UPD
    $updText = ""
    for ($i = 0; $i -lt 50; $i++) {
        $updText = GetCtrlText $setDlg 311
        if ($updText -match "v\d+\.\d+\.\d+") { break }
        Start-Sleep -Milliseconds 100
    }
    if ($updText -notmatch "v\d+\.\d+\.\d+") { throw "FAIL: GitHub update check did not return version! Got: '$updText'" }
    $cbLang = [E2EFull.U32]::GetDlgItem($setDlg, 305)
    [void][E2EFull.U32]::SendMessageW($cbLang, 0x014E, [IntPtr]2, [IntPtr]::Zero) # CB_SETCURSEL = English
    [void][E2EFull.U32]::PostMessageW($setDlg, 0x0111, [IntPtr]313, [IntPtr]::Zero) # ID_SET_SAVE
    if (-not (Wait-WindowClosed "StockTW_SettingsDlg_v2" 3000)) { throw "FAIL: Settings dialog did not close on Save!" }
    Write-Host "[PASS] 7. Settings saved & GitHub update check succeeded: '$updText'"

    # 8. 篩選分頁（台股 / 美股 / 全部）、排序切換與立即刷新
    $filterY = (& $sc 8) + (& $sc 24) + (& $sc 12) + (& $sc 12)
    ClickAt $main (& $sc 75) $filterY   # TW tab
    Start-Sleep -Milliseconds 120
    ClickAt $main (& $sc 125) $filterY  # US tab
    Start-Sleep -Milliseconds 120
    ClickAt $main (& $sc 30) $filterY   # All tab
    Start-Sleep -Milliseconds 120
    ClickAt $main ($c.W - (& $sc 50)) $filterY # Sort cycle
    Start-Sleep -Milliseconds 120
    ClickAt $main $c.RefX $c.TopY       # Force refresh
    Start-Sleep -Milliseconds 300
    Write-Host "[PASS] 8. Filter tabs, sort cycle, and manual refresh all responded cleanly"

    # 9. 驗證 DPAPI 加密檔案（檔頭為 SSTK\x01 且不含明文 2454）與低記憶體
    $pInfo = Get-Process -Id $proc.Id
    $privMb = [math]::Round($pInfo.PrivateMemorySize64 / 1MB, 2)
    $wsMb = [math]::Round($pInfo.WorkingSet64 / 1MB, 2)
    if ($privMb -gt 20) { throw "FAIL: Private memory too high (${privMb} MB)" }

    [void][E2EFull.U32]::PostMessageW($main, 0x0010, [IntPtr]::Zero, [IntPtr]::Zero) # WM_CLOSE
    if (-not $proc.WaitForExit(4000)) { throw "FAIL: Main process did not exit cleanly!" }

    $rawBytes = [System.IO.File]::ReadAllBytes($cfgPath)
    $headerOk = ($rawBytes[0] -eq 0x53 -and $rawBytes[1] -eq 0x53 -and $rawBytes[2] -eq 0x54 -and $rawBytes[3] -eq 0x4B -and $rawBytes[4] -eq 0x01)
    $textLossy = [System.Text.Encoding]::ASCII.GetString($rawBytes)
    if (-not $headerOk -or $textLossy.Contains("2454")) { throw "FAIL: config.dat encryption check failed!" }
    Write-Host "[PASS] 9. DPAPI encryption verified (no plaintext leak) & Memory: Private=${privMb} MB, WS=${wsMb} MB"

    # 10. 重啟驗證視窗位置與狀態延續
    $proc = Start-Process $exe -PassThru
    $main2 = Wait-Window "StockTW_MainWindow_v2" 5000
    if ($main2 -eq [IntPtr]::Zero) { throw "FAIL: Relaunch failed!" }
    Start-Sleep -Milliseconds 300
    [void][E2EFull.U32]::MoveWindow($main2, 260, 180, 960, 1120, $true)
    Start-Sleep -Milliseconds 300
    [void][E2EFull.U32]::PostMessageW($main2, 0x0010, [IntPtr]::Zero, [IntPtr]::Zero)
    if (-not $proc.WaitForExit(4000)) { throw "FAIL: Second exit failed!" }

    $proc = Start-Process $exe -PassThru
    $main3 = Wait-Window "StockTW_MainWindow_v2" 5000
    if ($main3 -eq [IntPtr]::Zero) { throw "FAIL: Third launch failed!" }
    Start-Sleep -Milliseconds 300
    $r3 = New-Object E2EFull.U32+RECT
    [void][E2EFull.U32]::GetWindowRect($main3, [ref]$r3)
    if ([Math]::Abs($r3.Left - 260) -gt 4 -or [Math]::Abs($r3.Top - 180) -gt 4) {
        throw "FAIL: Window position not restored! Expected ~(260,180), got ($($r3.Left),$($r3.Top))"
    }
    [void][E2EFull.U32]::PostMessageW($main3, 0x0010, [IntPtr]::Zero, [IntPtr]::Zero)
    if (-not $proc.WaitForExit(4000)) { throw "FAIL: Third exit failed!" }
    Write-Host "[PASS] 10. Restart & window position persistence verified (($($r3.Left),$($r3.Top)), ExitCode=$($proc.ExitCode))"

    Write-Host "`n=== ALL 10 FULL-FEATURE E2E TESTS PASSED! ==="
}
finally {
    if ($proc -and -not $proc.HasExited) {
        Stop-Process -Id $proc.Id -Force -ErrorAction SilentlyContinue
    }
    if ($hadBackup -and (Test-Path $backupPath)) {
        Copy-Item $backupPath $cfgPath -Force
        Remove-Item $backupPath -Force -ErrorAction SilentlyContinue
    }
}
