[Setup]
AppId={{8E9B7D4C-4B6E-4A2D-9C1F-51A2B3C4D5E6}
AppName=simple_stock
AppVersion=0.1.0
AppPublisher=tgsastudio
AppPublisherURL=https://github.com/tgsastudio/simple_stock
DefaultDirName={autopf}\simple_stock
DefaultGroupName=simple_stock
AllowNoIcons=yes
OutputDir=..\dist
OutputBaseFilename=simple_stock-Setup-x64-v0.1.0
Compression=lzma2/ultra64
SolidCompression=yes
WizardStyle=modern
PrivilegesRequired=lowest
PrivilegesRequiredOverridesAllowed=dialog
ArchitecturesInstallIn64BitMode=x64compatible

[Languages]
Name: "english"; MessagesFile: "compiler:Default.isl"

[Tasks]
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"
Name: "startup"; Description: "開機時自動啟動 simple_stock (Start with Windows)"; GroupDescription: "啟動選項:"

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
