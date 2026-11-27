# AGENT.md — Stock TW 開發與檢查流程規範

本文件記錄 **Stock TW (`tgsastudio/simple_stock`)** 的開發規範、本機測試協作流程、自動化檢查項目與發布流程。每次接手本專案時請務必遵守以下原則。

---

## 1. 專案定位與核心架構原則

- **技術棧**：純原生 Rust + **Windows 原生 Win32 GDI** 雙緩衝繪製（零 OpenGL / WebView / .NET / Python 依賴），支援 Windows 10 / 11 (x64)。
- **資源目標**：執行檔 `< 1 MB`、實際專用記憶體（Private RAM）`< 20 MB`（維持 ~6–8 MB）、待機 CPU 接近 `0%`。
- **不可破壞的產品規格**：
  - **雙模式介面**：僅保留「標準（100% 不透明）」與「浮動（25%–100% 透明度可調）」兩種模式，不加回側邊條模式（舊設定檔需維持 `#[serde(alias = "Sidebar")]` 相容）。
  - **跨模式一致功能**：標準與浮動模式皆須支援「全部 / 台股 / 美股」分頁切換與「總收益（TWD / USD）」摘要列。
  - **頂部按鈕視覺區隔**：`頂`（置頂）使用琥珀金色系、`刷`（刷新）使用青藍色系，深/淺色主題皆須清晰辨識。
  - **六國語系**：繁體中文 (`ZhTw`)、簡體中文 (`ZhCn`)、English (`En`)、日本語 (`Ja`)、한국어 (`Ko`)、Español (`Es`)，預設 `Auto` 自動跟隨系統語系；新增任何 UI 文字皆須補齊 6 種語系。
  - **單一執行個體**：透過 `Local\simple_stock_app_mutex` 限制只能開啟一個視窗；重複啟動時自動喚醒既有視窗（若已最小化則 `SW_RESTORE` 還原並帶至最前景）。
  - **視窗位置記憶**：包含標準模式與浮動模式獨立記憶、支援 Windows DWM 隱形邊框負座標、浮動 `✕` 須走 `WM_CLOSE`、`WM_EXITSIZEMOVE` 即時存檔。
  - **高 DPI 適配**：開發者實機為 **200% 縮放（192 DPI）**，所有介面、對話框、按鈕與文字排版皆須 Per-Monitor V2 DPI 感知，不得重疊或截斷。
  - **資料安全與保留**：所有設定與持股成本皆經 Windows DPAPI + 熵值加密儲存於 `%APPDATA%\tgsastudio\simple_stock\config\config.dat`；**嚴禁刪除或覆蓋使用者的個人 `config.dat`**。

---

## 2. 本機工具鏈與環境注意事項

- **Rust 工具鏈**：`stable-x86_64-pc-windows-gnu`（`%USERPROFILE%\.cargo\bin`）
- **MinGW-w64 (WinLibs)**：`$env:LOCALAPPDATA\Microsoft\WinGet\Packages\BrechtSanders.WinLibs*\mingw64\bin`（提供 `windres.exe`、`dlltool.exe` 等）
  - 每次執行 `cargo` 前，需將上述兩個路徑加入 `$env:PATH`。
- **Inno Setup 6**：`$env:LOCALAPPDATA\Programs\Inno Setup 6\ISCC.exe`
- **PowerShell 5.1**：串接指令請使用 `;` 搭配 `if ($LASTEXITCODE -eq 0)`，避免使用不支援的 `&&`。

---

## 3. 日常開發與實測協作流程

> **核心原則：異動完成後先讓使用者在本機實測確認，確認沒問題並明確指示後再進行封裝與發布（Release）。**

### UI 修改前置流程（先圖後改）

1. **先製作 UI 範例圖**：凡涉及介面外觀或排版調整，在修改程式碼之前，必須先產出 UI 範例圖供使用者檢視（未確認前**不得**先行實作）。
2. **微調須重出圖確認**：若使用者對範例圖提出修改意見，須依意見重新產出範例圖並再次確認，直到使用者明確同意（例如「ok」）才可開始實作。
3. **不擅動未提及的 UI**：實作時僅修改使用者明確指定並確認的項目，不得擅自更動其他既有的畫面樣式、標籤或排版。

### 實作與本機交付步驟

每次修改程式碼或 UI 後，依序執行以下步驟：

1. **編譯 Release 執行檔**
   ```powershell
   $mingw = (Get-ChildItem "$env:LOCALAPPDATA\Microsoft\WinGet\Packages\BrechtSanders.WinLibs*\mingw64\bin" -Directory | Select-Object -First 1).FullName
   $env:PATH = "$env:USERPROFILE\.cargo\bin;$mingw;$env:PATH"
   cargo build --release --offline
   ```
2. **執行自動化檢查（見第 4 節）**
   - 必須全部通過才可交付使用者實測。
3. **同步覆蓋至使用者本機安裝路徑**
   - 因 Agent 執行於非互動桌面，啟動的 GUI 不會出現在使用者畫面上；請先關閉執行中的 `simple_stock.exe`，再覆蓋本機安裝目錄：
   ```powershell
   taskkill /IM simple_stock.exe /F 2>$null
   Copy-Item -Path "D:\dev\stock widget\target\release\simple_stock.exe" -Destination "$env:LOCALAPPDATA\Programs\Stock TW\simple_stock.exe" -Force
   ```
4. **回報使用者實測**
   - 以繁體中文簡潔說明本次異動內容與 E2E 驗證結果，請使用者雙擊桌面 **Stock TW** 圖示親自確認。

---

## 4. 檢查與驗證流程（Verification Checklist）

### A. 自動化全功能 Win32 E2E 測試（必跑）
```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File "D:\dev\stock widget\tests\e2e_win32.ps1"
```
> 腳本會自動備份並在結束時完整還原使用者的 `config.dat`。

目前涵蓋 10 項必檢項目：
1. 新啟動視窗可視與高 DPI（192 DPI）尺寸檢查
2. 雙模式切換（`標準 ↔ 浮動`）
3. 視窗置頂切換（`Always on Top`）
4. 新增自選股（含持股股數與成本）
5. 線上關鍵字搜尋與 `Esc` 關閉對話框
6. 編輯持股與即時收益預覽計算
7. 設定視窗四分頁（一般 / 外觀 / 刷新 / 關於）切換、Windows 開機自動啟動登錄檔寫入與移除、GitHub 最新版本檢查與儲存
8. 市場分頁切換與手動刷新
9. DPAPI 加密驗證（無明文外洩）與記憶體檢核（`Private RAM < 20 MB`）
10. 重啟後視窗位置還原 + 單一執行個體檢查（重複啟動立即退出）

- 若新增重要功能或修正回歸 Bug，應同步在 `tests/e2e_win32.ps1` 補上對應斷言。

### B. 單元測試（報價 / DPAPI / 收益邏輯有變動時必跑）
```powershell
cargo test -- --nocapture
```

---

## 5. 封裝與發布流程（Release）

> 僅在使用者明確要求（例如「可以 release」、「release 吧」）時執行。

1. **同步更新版本號**（例如 `1.2.1`）：
   - `Cargo.toml`（`version`）
   - `assets/app.rc`（`FILEVERSION`、`PRODUCTVERSION`、`FileVersion`、`ProductVersion`）
   - `installer/simple_stock.iss`（`AppVersion`、`VersionInfoVersion`、`OutputBaseFilename`）
   - `README.md`（如有功能異動與 `SHA256` 範例檔名，務必一併更新）
2. **本機建置與安裝檔驗證**：
   - 執行 `cargo build --release --offline`
   - 執行 `& "$env:LOCALAPPDATA\Programs\Inno Setup 6\ISCC.exe" /Q "installer\simple_stock.iss"` 確認 `dist\StockTW-Setup-x64-vX.Y.Z.exe` 編譯成功
   - 視需要補跑一次 E2E 驗證並同步 `.exe` 至本機安裝路徑
3. **Commit、Tag 與推送觸發 CI**：
   ```powershell
   git add ...
   git commit -m "feat/fix: vX.Y.Z - ..."
   git tag vX.Y.Z
   git push origin main vX.Y.Z
   ```
4. **等待 GitHub Actions 完成並更新 Release Notes**：
   - `.github/workflows/release.yml` 會以 Windows MSVC 自動編譯、打包安裝檔、產生 `SHA256SUMS.txt` 並建立 GitHub Release。
   - **資產規範**：僅提供 Windows 安裝檔（`StockTW-Setup-x64-vX.Y.Z.exe`）與 `SHA256SUMS.txt`，**不提供** Portable 免安裝版本。
   - 監控 CI 完成（`gh run watch <run_id> --exit-status`）後，使用 `gh release edit vX.Y.Z` 撰寫簡潔清楚的繁體中文 Release Notes，並確認資產清單完整。

---

## 6. 溝通規範

- 全程使用**繁體中文**回覆。
- 回覆保持精簡、重點條列，不重複貼出冗長內容。
