# Microsoft Store 免費上架指南（Stock TW）

透過 Microsoft Store 發布 **MSIX** 套件，**個人開發者完全免費（$0）**，且通過審核後由 **Microsoft 官方憑證自動簽章**，使用者安裝時零 SmartScreen 警告並享有自動更新。

---

## 步驟 1：免費註冊微軟開發者帳號
1. 前往微軟新版開發者入口（務必使用此連結，免信用卡、免註冊費）：
   - https://storedeveloper.microsoft.com
2. 登入個人 Microsoft 帳號，依指示使用政府核發證件（護照／駕照／身分證）與手機自拍完成身分驗證。

---

## 步驟 2：建立應用程式並取得 Product Identity
1. 登入 **Microsoft Partner Center** → **Apps and games** → **New product** → **MSIX or PWA app**。
2. 保留應用程式名稱（例如 `Stock TW` 或 `Stock TW - 台美股看盤`）。
3. 進入該產品頁左側 **Product management** → **Product Identity**，記下這三個值：
   - `Package/Identity/Name`（例如 `12345YourName.StockTW`）
   - `Package/Identity/Publisher`（例如 `CN=XXXXXXXX-XXXX-XXXX-XXXX-XXXXXXXXXXXX`）
   - `Package/Properties/PublisherDisplayName`（你的開發者顯示名稱）

---

## 步驟 3：一鍵產出 MSIX 套件（無須安裝 Windows SDK）
1. 打開 GitHub Repo 的 **Actions** 分頁 → 選擇左側 **Build Microsoft Store MSIX**。
2. 點擊 **Run workflow**，貼上步驟 2 取得的三個值並執行（約 1 分鐘完成）。
3. 完成後從該次 Run 的 **Artifacts** 下載 `stock-tw-msix` 解壓縮，即取得 `StockTW-x64-v*.msix`（Microsoft Store 直接接受未簽章的 MSIX，審核通過後微軟會自動以官方憑證簽章）。

---

## 步驟 4：送出審核（可直接複製貼上）
在 Partner Center 建立新 Submission：
- **Packages**：上傳 `StockTW-x64-v*.msix`。
- **Store listings → Store logos**：可直接使用本專案已產出的 `msix/Assets/StoreListing300x300.png`（300×300 PNG）與 1–2 張實機截圖。
- **Privacy policy URL**：
  ```
  https://github.com/tgsastudio/simple_stock#privacy-notice--license
  ```
- **Notes for certification（`runFullTrust` 權限說明，直接貼上即可）**：
  ```
  Stock TW is an open-source native Win32 GDI desktop application written in Rust.
  It requires runFullTrust to render a lightweight Win32 window (including an optional floating desktop widget) and to store user watchlist/holdings locally using Windows DPAPI encryption (CryptProtectData).
  No login is required; the app only makes read-only HTTPS requests to public stock quote endpoints (TWSE/TPEx and Yahoo Finance).
  Source code: https://github.com/tgsastudio/simple_stock
  ```
- 點擊 **Submit to the Store**，通常 1–3 個工作天完成審核上架。
