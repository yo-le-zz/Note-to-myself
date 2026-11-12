; Installateur Windows (Inno Setup). Compilé par le workflow de release :
;   iscc /DAppVersion=1.1.0 packaging\windows\note-to-myself.iss
#ifndef AppVersion
  #define AppVersion "0.0.0"
#endif

[Setup]
AppId={{6B0B2E34-5F0B-4C57-9C0E-7A1E2D4B8F11}
AppName=Note to Myself
AppVersion={#AppVersion}
AppPublisher=yolezz
DefaultDirName={autopf}\Note to Myself
DefaultGroupName=Note to Myself
OutputDir=..\..\dist
OutputBaseFilename=note-to-myself-{#AppVersion}-windows-setup
SetupIconFile=note-to-myself.ico
UninstallDisplayIcon={app}\note-to-myself.exe
Compression=lzma2
SolidCompression=yes
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
PrivilegesRequired=lowest
PrivilegesRequiredOverridesAllowed=dialog
WizardStyle=modern

[Files]
Source: "..\..\target\release\note-to-myself.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\..\target\release\ntm-plugin-check.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\..\LICENSE"; DestDir: "{app}"; Flags: ignoreversion

[Icons]
Name: "{group}\Note to Myself"; Filename: "{app}\note-to-myself.exe"
Name: "{autodesktop}\Note to Myself"; Filename: "{app}\note-to-myself.exe"; Tasks: desktopicon

[Tasks]
Name: "desktopicon"; Description: "Create a desktop shortcut"; Flags: unchecked

[Run]
Filename: "{app}\note-to-myself.exe"; Description: "Launch Note to Myself"; Flags: nowait postinstall skipifsilent
