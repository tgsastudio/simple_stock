[Setup]
AppId={{8E9B7D4C-4B6E-4A2D-9C1F-51A2B3C4D5E6}
AppName=Stock TW
AppVersion=1.3.4
VersionInfoVersion=1.3.4.0
AppPublisher=tgsastudio
AppPublisherURL=https://github.com/tgsastudio/simple_stock
DefaultDirName={autopf}\Stock TW
DefaultGroupName=Stock TW
AllowNoIcons=yes
OutputDir=..\dist
OutputBaseFilename=StockTW-Setup-x64-v1.3.4
SetupIconFile=..\assets\icon.ico
UninstallDisplayIcon={app}\simple_stock.exe
Compression=lzma2
SolidCompression=yes
WizardStyle=modern
PrivilegesRequired=lowest
PrivilegesRequiredOverridesAllowed=dialog
ArchitecturesInstallIn64BitMode=x64compatible
ShowLanguageDialog=auto
; Seamless in-place upgrade via Windows Restart Manager (no taskkill heuristics)
AppMutex=Local\simple_stock_app_mutex
UsePreviousAppDir=yes
UsePreviousGroup=yes
UsePreviousTasks=yes
UsePreviousLanguage=yes
CloseApplications=yes
CloseApplicationsFilter=*.exe
RestartApplications=no

[Languages]
Name: "chinesetraditional"; MessagesFile: "ChineseTraditional.isl"
Name: "english"; MessagesFile: "compiler:Default.isl"

[CustomMessages]
english.StartupTask=Start Stock TW automatically when Windows starts
english.StartupGroup=Startup Options:
english.UpgradeNote=An existing version of Stock TW was detected (%1). Continuing will upgrade in-place while keeping your watchlist and settings.
english.DeleteDataPrompt=Do you also want to delete your saved watchlist, holdings, and settings?%n%nChoose "No" to keep your data so it will be automatically loaded if you reinstall Stock TW later.
chinesetraditional.StartupTask=開機時自動啟動 Stock TW
chinesetraditional.StartupGroup=啟動選項:
chinesetraditional.UpgradeNote=偵測到已安裝舊版 Stock TW (%1)。繼續安裝將會直接覆蓋升級，並完整保留您的自選股與持股設定。
chinesetraditional.DeleteDataPrompt=是否要一併刪除已儲存的自選股、持股成本與個人化設定？%n%n選擇「否」將會保留您的加密資料，未來重新安裝 Stock TW 時會自動載入。

[Tasks]
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"
Name: "startup"; Description: "{cm:StartupTask}"; GroupDescription: "{cm:StartupGroup}"

[Files]
Source: "..\target\release\simple_stock.exe"; DestDir: "{app}"; Flags: ignoreversion

[Icons]
Name: "{group}\Stock TW"; Filename: "{app}\simple_stock.exe"
Name: "{group}\{cm:UninstallProgram,Stock TW}"; Filename: "{uninstallexe}"
Name: "{autodesktop}\Stock TW"; Filename: "{app}\simple_stock.exe"; Tasks: desktopicon

[Registry]
Root: HKCU; Subkey: "Software\Microsoft\Windows\CurrentVersion\Run"; ValueType: string; ValueName: "Stock TW"; ValueData: """{app}\simple_stock.exe"""; Flags: uninsdeletevalue; Tasks: startup

[Run]
Filename: "{app}\simple_stock.exe"; Description: "{cm:LaunchProgram,Stock TW}"; Flags: nowait postinstall skipifsilent

[Code]
function GetInstalledVersion(): String;
var
  UninstallKey: String;
  Ver: String;
begin
  Result := '';
  UninstallKey := 'Software\Microsoft\Windows\CurrentVersion\Uninstall\{8E9B7D4C-4B6E-4A2D-9C1F-51A2B3C4D5E6}_is1';
  if RegQueryStringValue(HKCU, UninstallKey, 'DisplayVersion', Ver) then
    Result := Ver
  else if RegQueryStringValue(HKLM, UninstallKey, 'DisplayVersion', Ver) then
    Result := Ver;
end;

procedure InitializeWizard();
var
  PrevVer: String;
begin
  PrevVer := GetInstalledVersion();
  if PrevVer <> '' then
  begin
    WizardForm.WelcomeLabel2.Caption :=
      FmtMessage(CustomMessage('UpgradeNote'), [PrevVer]) + #13#10#13#10 +
      WizardForm.WelcomeLabel2.Caption;
  end;
end;

procedure CurUninstallStepChanged(CurUninstallStep: TUninstallStep);
var
  AppDataDir: String;
  VendorDir: String;
  PortableDat: String;
  PortableJson: String;
begin
  if (CurUninstallStep = usPostUninstall) and (not UninstallSilent) then
  begin
    AppDataDir := ExpandConstant('{userappdata}\tgsastudio\simple_stock');
    VendorDir := ExpandConstant('{userappdata}\tgsastudio');
    PortableDat := ExpandConstant('{app}\config.dat');
    PortableJson := ExpandConstant('{app}\config.json');

    if DirExists(AppDataDir) or FileExists(PortableDat) or FileExists(PortableJson) then
    begin
      if MsgBox(CustomMessage('DeleteDataPrompt'), mbConfirmation, MB_YESNO or MB_DEFBUTTON2) = IDYES then
      begin
        if DirExists(AppDataDir) then
          DelTree(AppDataDir, True, True, True);
        if DirExists(VendorDir) then
          RemoveDir(VendorDir);
        if FileExists(PortableDat) then
          DeleteFile(PortableDat);
        if FileExists(PortableJson) then
          DeleteFile(PortableJson);
      end;
    end;
  end;
end;
