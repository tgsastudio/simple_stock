[Setup]
AppId={{8E9B7D4C-4B6E-4A2D-9C1F-51A2B3C4D5E6}
AppName=simple_stock
AppVersion=0.3.0
AppPublisher=tgsastudio
AppPublisherURL=https://github.com/tgsastudio/simple_stock
DefaultDirName={autopf}\simple_stock
DefaultGroupName=simple_stock
AllowNoIcons=yes
OutputDir=..\dist
OutputBaseFilename=simple_stock-Setup-x64-v0.3.0
SetupIconFile=..\assets\icon.ico
UninstallDisplayIcon={app}\simple_stock.exe
Compression=lzma2/ultra64
SolidCompression=yes
WizardStyle=modern
PrivilegesRequired=lowest
PrivilegesRequiredOverridesAllowed=dialog
ArchitecturesInstallIn64BitMode=x64compatible
ShowLanguageDialog=auto

[Languages]
Name: "chinesetraditional"; MessagesFile: "ChineseTraditional.isl"
Name: "english"; MessagesFile: "compiler:Default.isl"

[CustomMessages]
english.StartupTask=Start simple_stock automatically when Windows starts
english.StartupGroup=Startup Options:
chinesetraditional.StartupTask=開機時自動啟動 simple_stock
chinesetraditional.StartupGroup=啟動選項:

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
