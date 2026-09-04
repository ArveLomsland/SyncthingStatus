; Inno Setup-skript for SyncthingStatus
; Bygg med: packaging\windows\build-installer.ps1
;   eller:  ISCC.exe /DMyAppVersion=0.1.0 packaging\windows\syncthing-status.iss

#define MyAppName "SyncthingStatus"
#ifndef MyAppVersion
  #define MyAppVersion "0.1.0"
#endif
#define MyAppPublisher "Arve"
#define MyAppExeName "syncthing-status.exe"
#define SourceExe "..\..\target\release\" + MyAppExeName

[Setup]
AppId={{7C4B9E2A-3F51-4D8B-9A6E-1B2C3D4E5F60}
AppName={#MyAppName}
AppVersion={#MyAppVersion}
AppPublisher={#MyAppPublisher}
VersionInfoVersion={#MyAppVersion}
; Per bruker: trenger ikke administrator
PrivilegesRequired=lowest
DefaultDirName={localappdata}\Programs\SyncthingStatus
DefaultGroupName={#MyAppName}
DisableProgramGroupPage=yes
DisableDirPage=auto
UninstallDisplayIcon={app}\{#MyAppExeName}
UninstallDisplayName={#MyAppName}
OutputDir=..\..\dist
OutputBaseFilename=syncthing-status-{#MyAppVersion}-setup
Compression=lzma2/max
SolidCompression=yes
WizardStyle=modern
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible

[Languages]
Name: "no"; MessagesFile: "compiler:Languages\Norwegian.isl"
Name: "en"; MessagesFile: "compiler:Default.isl"

[Tasks]
Name: "startup"; Description: "Start automatisk når jeg logger inn"; GroupDescription: "Tillegg:"

[Files]
Source: "{#SourceExe}"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\..\README.md"; DestDir: "{app}"; Flags: ignoreversion

[Icons]
Name: "{group}\{#MyAppName}"; Filename: "{app}\{#MyAppExeName}"
Name: "{group}\Avinstaller {#MyAppName}"; Filename: "{uninstallexe}"
Name: "{userstartup}\{#MyAppName}"; Filename: "{app}\{#MyAppExeName}"; Tasks: startup

[Run]
Filename: "{app}\{#MyAppExeName}"; Description: "Start {#MyAppName} nå"; Flags: nowait postinstall skipifsilent

[UninstallRun]
Filename: "{sys}\taskkill.exe"; Parameters: "/IM {#MyAppExeName} /F"; Flags: runhidden skipifdoesntexist; RunOnceId: "KillTray"

[Code]
// Tray-appen har ingen vinduer, så CloseApplications finner den ikke.
// Avslutt en eventuell kjørende instans før filene byttes ut.
function PrepareToInstall(var NeedsRestart: Boolean): String;
var
  ResultCode: Integer;
begin
  Exec(ExpandConstant('{sys}\taskkill.exe'), '/IM {#MyAppExeName} /F',
       '', SW_HIDE, ewWaitUntilTerminated, ResultCode);
  Result := '';
end;
