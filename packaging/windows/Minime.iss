#ifndef MyAppVersion
  #error MyAppVersion is required
#endif
#ifndef MyAppArch
  #error MyAppArch is required
#endif
#ifndef MyAppBinary
  #error MyAppBinary is required
#endif
#ifndef MyOutputDir
  #error MyOutputDir is required
#endif

#define MyAppName "Minime"
#define MyAppPublisher "scorpion7slayer"
#define MyAppUrl "https://github.com/scorpion7slayer/Minime"

[Setup]
AppId={{8533C99A-68AB-436D-8057-77DDD59C6616}
AppName={#MyAppName}
AppVersion={#MyAppVersion}
AppPublisher={#MyAppPublisher}
AppPublisherURL={#MyAppUrl}
AppSupportURL={#MyAppUrl + "/issues"}
AppUpdatesURL={#MyAppUrl + "/releases"}
DefaultDirName={localappdata}\Programs\Minime
DefaultGroupName=Minime
DisableProgramGroupPage=yes
LicenseFile={#SourcePath}\..\..\LICENSE
OutputDir={#MyOutputDir}
OutputBaseFilename=Minime-{#MyAppVersion}-windows-{#MyAppArch}-setup
SetupIconFile={#SourcePath}\..\..\assets\minime.ico
UninstallDisplayIcon={app}\minime.exe
Compression=lzma2
SolidCompression=yes
WizardStyle=modern
PrivilegesRequired=lowest
CloseApplications=no
RestartApplications=no
VersionInfoVersion={#MyAppVersion}.0
VersionInfoCompany={#MyAppPublisher}
VersionInfoDescription=Minime image compressor and converter
VersionInfoProductName={#MyAppName}
VersionInfoProductVersion={#MyAppVersion}

#if MyAppArch == "x64"
ArchitecturesAllowed=x64compatible and not arm64
ArchitecturesInstallIn64BitMode=x64compatible
#elif MyAppArch == "arm64"
ArchitecturesAllowed=arm64
ArchitecturesInstallIn64BitMode=arm64
#else
  #error MyAppArch must be x64 or arm64
#endif

[Files]
Source: "{#MyAppBinary}"; DestDir: "{app}"; DestName: "minime.exe"; Flags: ignoreversion
Source: "{#SourcePath}\..\..\assets\minime.ico"; DestDir: "{app}"; Flags: ignoreversion

[Tasks]
Name: "desktopicon"; Description: "Create a desktop shortcut"; GroupDescription: "Additional shortcuts:"; Flags: unchecked

[Icons]
Name: "{group}\Minime"; Filename: "{app}\minime.exe"; IconFilename: "{app}\minime.ico"
Name: "{autodesktop}\Minime"; Filename: "{app}\minime.exe"; IconFilename: "{app}\minime.ico"; Tasks: desktopicon

[Run]
Filename: "{app}\minime.exe"; Description: "Launch Minime"; Flags: nowait postinstall skipifsilent
