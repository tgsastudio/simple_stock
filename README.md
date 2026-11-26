# Stock TW

輕量化 Windows 即時看盤 Dashboard 軟體（支援台股與美股），以純原生 Rust + **Windows 原生 Win32 GDI** 開發，無須登入、零外部執行環境依賴。

## 下載安裝
請至 [GitHub Releases](https://github.com/tgsastudio/simple_stock/releases/latest) 下載最新版本的 Windows 安裝檔（`StockTW-Setup-x64-v*.exe`）：
- 支援 **繁體中文 / English** 自動跟隨系統語系的安裝與解除安裝精靈。
- 支援**舊版無痛覆蓋升級**，自動保留已儲存的自選股、持股成本與個人化設定。
- 解除安裝時可自由選擇**保留或刪除**個人設定資料（選擇保留後，未來重新安裝會自動載入）。

## 核心特色
- **極致輕量化與省電**：
  - 純原生 Win32 GDI 雙緩衝繪製，無 OpenGL / WebView / .NET / Python 開銷。
  - 核心執行檔僅約 **0.85 MB**、實際專用記憶體（Private RAM）僅約 **6–7 MB**、待機 CPU 接近 **0%**。
  - 內嵌 Windows Common Controls v6 與 Per-Monitor V2 高 DPI 感知，支援深色/淺色主題。
- **免登入即時報價（台股 + 美股）**：
  - **台股（上市 / 上櫃 / ETF）**：優先串接證交所 / 櫃買中心公開即時 API 單次批次查詢，查無資料時自動切換 Yahoo Finance 備援。
  - **美股（NASDAQ / NYSE / ETF / 指數）**：串接 Yahoo Finance 公開報價端點。
- **三種介面形態與完整狀態記憶**：
  - 支援 **標準 Dashboard**、**精簡側邊條**、**桌面透明浮動小工具**（無邊框、透明度 25%–100% 可調、支援視窗置頂與拖曳移動）一鍵切換。
  - 所有模式皆支援 **全部 / 台股 / 美股** 分頁切換與 **總收益（TWD / USD）** 摘要列。
  - 自動記憶各模式的視窗大小與螢幕位置（含多螢幕邊界保護）、上次所在分頁與排序方式。
- **自選股管理與持股收益計算**：
  - 支援直接輸入代碼或線上關鍵字搜尋加入自選股，並可透過滑鼠拖曳（`☰`）或右鍵選單調整排序。
  - 每檔自選股皆可點擊「持股 / Pos」、雙擊卡片或右鍵設定 **持有數量** 與 **持有成本**，依現價即時計算單檔與總投資組合的 **收益金額** 與 **收益比例 (%)**。
- **智慧省電刷新方案**：
  - 提供 `5 秒（極速）` / `15 秒（均衡）` / `30 秒（省電）` / `60 秒（超省電）` / `手動` 五種刷新模式。
  - 內建台美股開盤時段感知（休市自動停止輪詢）、視窗最小化自動暫停、以及網路異常自動退避。
- **安全性與多語系**：
  - 所有自選股與持股成本皆透過 **Windows 原生 DPAPI (`CryptProtectData`) + 應用程式專屬熵值** 加密儲存於 `config.dat`，防止其他程式或使用者讀取。
  - 支援 **繁體中文 / English** 即時切換（預設自動跟隨 Windows 作業系統語言），並內建 **檢查更新** 功能自動比對 GitHub Releases。

## 建置與測試（開發者）
```powershell
# 編譯 Release 最佳化版本
cargo build --release

# 執行單元測試（DPAPI 加解密 + 台美股即時報價與收益計算）
cargo test -- --nocapture

# 執行 10 項全功能自動化 Win32 E2E 測試（自動備份與還原個人設定檔）
powershell -NoProfile -ExecutionPolicy Bypass -File tests\e2e_win32.ps1

# 使用 Inno Setup 打包 Windows 安裝檔（輸出至 dist/）
ISCC.exe installer\simple_stock.iss
```

## Code Signing Policy & Privacy Notice

Free code signing provided by [SignPath.io](https://signpath.io/), certificate by [SignPath Foundation](https://signpath.org/).

- **Committers & Approvers**: Repository maintainers of `tgsastudio/simple_stock`.
- **Privacy**: This program will **not** transfer any user information, watchlist, holdings, or personal data to other networked systems. All user preferences and holdings are stored exclusively on the local device and encrypted using Windows DPAPI. The application only performs read-only HTTPS requests to public market quote endpoints (TWSE/TPEx, Yahoo Finance) and GitHub Releases (when manually checking for updates).
- **License**: Released under the [MIT License](LICENSE).
