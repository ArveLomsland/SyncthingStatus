; Inno Setup script for SyncthingStatus
; Build with: packaging\windows\build-installer.ps1
;         or: ISCC.exe /DMyAppVersion=0.1.0 packaging\windows\syncthing-status.iss

#define MyAppName "SyncthingStatus"
#ifndef MyAppVersion
  #define MyAppVersion "0.1.0"
#endif
#define MyAppPublisher "Arve Lomsland"
#define MyAppUrl "https://github.com/ArveLomsland/SyncthingStatus"
#define MyAppExeName "syncthing-status.exe"
#define SourceExe "..\..\target\release\" + MyAppExeName

[Setup]
AppId={{7C4B9E2A-3F51-4D8B-9A6E-1B2C3D4E5F60}
AppName={#MyAppName}
AppVersion={#MyAppVersion}
AppPublisher={#MyAppPublisher}
AppPublisherURL={#MyAppUrl}
AppSupportURL={#MyAppUrl}/issues
AppUpdatesURL={#MyAppUrl}/releases
VersionInfoVersion={#MyAppVersion}
; Per-user install: no administrator rights required
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
Name: "en"; MessagesFile: "compiler:Default.isl"
Name: "nb"; MessagesFile: "compiler:Languages\Norwegian.isl"

[Tasks]
Name: "startup"; Description: "Start automatically when I log in"; GroupDescription: "Additional options:"

[Files]
Source: "{#SourceExe}"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\..\README.md"; DestDir: "{app}"; Flags: ignoreversion

[Icons]
Name: "{group}\{#MyAppName}"; Filename: "{app}\{#MyAppExeName}"
Name: "{group}\Uninstall {#MyAppName}"; Filename: "{uninstallexe}"
Name: "{userstartup}\{#MyAppName}"; Filename: "{app}\{#MyAppExeName}"; Tasks: startup

[Run]
Filename: "{app}\{#MyAppExeName}"; Description: "Launch {#MyAppName} now"; Flags: nowait postinstall skipifsilent

[UninstallRun]
Filename: "{sys}\taskkill.exe"; Parameters: "/IM {#MyAppExeName} /F"; Flags: runhidden skipifdoesntexist; RunOnceId: "KillTray"

[Code]
// The tray app has no windows, so CloseApplications cannot detect it.
// Terminate any running instance before the files are replaced.
function PrepareToInstall(var NeedsRestart: Boolean): String;
var
  ResultCode: Integer;
begin
  Exec(ExpandConstant('{sys}\taskkill.exe'), '/IM {#MyAppExeName} /F',
       '', SW_HIDE, ewWaitUntilTerminated, ResultCode);
  Result := '';
end;
