#ifndef MyAppVersion
  #define MyAppVersion "1.0.0"
#endif

#ifndef BuildOutputDir
  #define BuildOutputDir "..\dist\windows\SimpleSteelCalculator"
#endif

#define MyAppName "Simple Steel Calculator"
#define MyAppPublisher "Harbor Pipe & Steel Inc."
#define MyAppURL "https://www.harborpipe.com/"
#define MyAppExeName "SimpleSteelCalculator.exe"
#define MyAppSetupName "SimpleSteelCalculator-" + MyAppVersion + "-x64-Setup"
#define MyAppIcon "..\logo.ico"
#define SignToolArgs GetEnv("STEELCAL_SIGN_ARGS")

#if SignToolArgs != ""
  #define SigningEnabled
#endif

[Setup]
AppId={{4C27B641-9105-4A66-B06B-BCB523A46EEE}
AppName={#MyAppName}
AppVersion={#MyAppVersion}
AppPublisher={#MyAppPublisher}
AppPublisherURL={#MyAppURL}
AppSupportURL={#MyAppURL}
AppUpdatesURL={#MyAppURL}
DefaultDirName={autopf}\{#MyAppName}
DefaultGroupName={#MyAppName}
DisableProgramGroupPage=yes
UsePreviousAppDir=yes
UsePreviousTasks=yes
UninstallDisplayIcon={app}\{#MyAppExeName}
PrivilegesRequired=admin
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
Compression=lzma2
SolidCompression=yes
WizardStyle=modern
OutputDir=Output
OutputBaseFilename={#MyAppSetupName}
SetupIconFile={#MyAppIcon}
VersionInfoVersion={#MyAppVersion}
VersionInfoCompany={#MyAppPublisher}
VersionInfoDescription={#MyAppName} Installer
VersionInfoCopyright={#MyAppPublisher}
#ifdef SigningEnabled
SignTool=steelcal {#SignToolArgs} $f
SignedUninstaller=yes
#endif

[Languages]
Name: "english"; MessagesFile: "compiler:Default.isl"

[Tasks]
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"; Flags: unchecked

[Files]
#ifdef SigningEnabled
Source: "{#BuildOutputDir}\{#MyAppExeName}"; DestDir: "{app}"; Flags: ignoreversion signonce
#else
Source: "{#BuildOutputDir}\{#MyAppExeName}"; DestDir: "{app}"; Flags: ignoreversion
#endif
Source: "{#BuildOutputDir}\*"; DestDir: "{app}"; Flags: ignoreversion recursesubdirs createallsubdirs; Excludes: "{#MyAppExeName}"

[Icons]
Name: "{group}\{#MyAppName}"; Filename: "{app}\{#MyAppExeName}"
Name: "{autodesktop}\{#MyAppName}"; Filename: "{app}\{#MyAppExeName}"; Tasks: desktopicon

[Run]
Filename: "{app}\{#MyAppExeName}"; Description: "{cm:LaunchProgram,{#MyAppName}}"; Flags: nowait postinstall skipifsilent
