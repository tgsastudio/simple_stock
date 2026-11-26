[Setup]
AppId={{8E9B7D4C-4B6E-4A2D-9C1F-51A2B3C4D5E6}
AppName=simple_stock
AppVersion=0.3.2
VersionInfoVersion=0.3.2.0
AppPublisher=tgsastudio
AppPublisherURL=https://github.com/tgsastudio/simple_stock
DefaultDirName={autopf}\simple_stock
DefaultGroupName=simple_stock
AllowNoIcons=yes
OutputDir=..\dist
OutputBaseFilename=simple_stock-Setup-x64-v0.3.2
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
english.StartupTask=Start simple_stock automatically when Windows starts
english.StartupGroup=Startup Options:
english.UpgradeNote=An existing version of simple_stock was detected (%1). Continuing will upgrade in-place while keeping your watchlist and settings.
chinesetraditional.StartupTask=開機時自動啟動 simple_stock
chinesetraditional.StartupGroup=啟動選項:
chinesetraditional.UpgradeNote=偵測到已安裝舊版 simple_stock (%1)。繼續安裝將會直接覆蓋升級，並完整保留您的自選股與持股設定。

[Tasks]
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"
Name: "startup"; Description: "{cm:StartupTask}"; GroupDescription: "{cm:StartupGroup}"

[Files]
Source: "..\target\release\simple_stock.exe"; DestDir: "{app}"; Flags: ignoreversion

[Icons]
Name: "{group}\simple_stock"; Filename: "{app}\simple_stock.exe"
Name: "{group}\{cm:UninstallProgram,simple_stock}"; Filename: "{uninstallexe}"
Name: "{autodesktop}\simple_stock"; Filename: "{app}\simple_stock.exe"; Tasks: desktopicon

[Registry]
Root: HKCU; Subkey: "Software\Microsoft\Windows\CurrentVersion\Run"; ValueType: string; ValueName: "simple_stock"; ValueData: """{app}\simple_stock.exe"""; Flags: uninsdeletevalue; Tasks: startup

[Run]
Filename: "{app}\simple_stock.exe"; Description: "{cm:LaunchProgram,simple_stock}"; Flags: nowait postinstall skipifsilent

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
