# simple_stock

輕量化 Windows 即時看盤 Dashboard 軟體（支援台股與美股），以純原生 Rust (`eframe` / `egui`) 開發。

## 特色
- **極致輕量**：單一原生執行檔約 **3.25 MB**（無需安裝 .NET / Java / Python / 瀏覽器核心），待機 CPU 接近 0%。
- **免登入即時報價**：
  - 台股（上市 / 上櫃 / ETF）：優先串接證交所 / 櫃買中心公開即時 API 批次查詢，並具 Yahoo Finance 自動備援。
  - 美股（NASDAQ / NYSE / ETF / 指數）：串接 Yahoo Finance 公開報價端點。
- **三種介面形態一鍵切換**：
  - **標準模式**：完整 Dashboard、市場篩選（全部 / 台股 / 美股）、滑鼠拖曳排序（`☰`）與漲跌幅排序檢視。
  - **精簡側邊條模式**：窄條迷你列表，不佔工作空間。
  - **桌面透明浮動小工具**：無邊框、透明度可調（25%–100%）、可拖曳標題列移動並支援視窗置頂（Always on Top）。
- **刷新頻率與智慧省電**：
  - 提供 `5 秒` / `15 秒` / `30 秒` / `60 秒` / `手動` 五種刷新策略。
  - 內建雙市場開盤時段感知（休市自動停止輪詢）、視窗最小化暫停刷新、網路異常自動退避。
- **自選股管理**：
  - 支援直接輸入代碼或線上關鍵字搜尋加入。
  - 支援拖曳排序與右鍵選單（移至最頂 / 上移 / 下移 / 移除）。
  - 本機 JSON 儲存（預設位於 `%APPDATA%\tgsastudio\simple_stock\config.json`；若在 `.exe` 同目錄放置 `config.json` 則自動啟用可攜模式）。

## 建置與打包（開發者）
```powershell
# 編譯 Release 最佳化版本
cargo build --release

# 使用 Inno Setup 打包 Windows 安裝檔（輸出至 dist/）
ISCC.exe installer\simple_stock.iss
```
