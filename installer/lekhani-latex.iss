; Lekhani Latex - Inno Setup installer
;
; Compiled by .github/workflows/release.yml from the project root, for example:
;   ISCC.exe /DMyAppVersion=0.0.4 /DMyVersionInfo=0.0.4.0 /DMyArch=x64 ^
;            /DMySourceExe=...\target\x86_64-pc-windows-msvc\release\lekhani-latex.exe ^
;            /DMyOutputBaseFilename=LekhaniLatex-Setup-0.0.4-x64 installer\lekhani-latex.iss

#ifndef MyAppVersion
  #define MyAppVersion "0.0.0"
#endif
#ifndef MyVersionInfo
  #define MyVersionInfo "0.0.0.0"
#endif
#ifndef MyArch
  #define MyArch "x64"
#endif
#ifndef MySourceExe
  #define MySourceExe "..\target\release\lekhani-latex.exe"
#endif
#ifndef MyOutputBaseFilename
  #define MyOutputBaseFilename "LekhaniLatex-Setup"
#endif

[Setup]
AppId={{9F3B4D2A-7C1E-4B6A-8D5F-3E2A1C0B9D47}
AppName=Lekhani Latex
AppVersion={#MyAppVersion}
AppVerName=Lekhani Latex {#MyAppVersion}
AppPublisher=Borneel Bikash Phukan
AppPublisherURL=https://borneelphukan.github.io/lekhani-latex/
AppSupportURL=https://github.com/borneelphukan/lekhani-latex/issues
AppUpdatesURL=https://github.com/borneelphukan/lekhani-latex/releases
DefaultDirName={autopf}\Lekhani Latex
DefaultGroupName=Lekhani Latex
DisableProgramGroupPage=yes
OutputDir=..\target
OutputBaseFilename={#MyOutputBaseFilename}
SetupIconFile=..\assets\logo.ico
UninstallDisplayIcon={app}\lekhani-latex.exe
Compression=lzma2
SolidCompression=yes
WizardStyle=modern
CloseApplications=yes
RestartApplications=no
VersionInfoVersion={#MyVersionInfo}
VersionInfoProductVersion={#MyVersionInfo}
VersionInfoProductName=Lekhani Latex
VersionInfoCompany=Borneel Bikash Phukan
VersionInfoDescription=Lekhani Latex Setup
#if MyArch == "x64"
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
#elif MyArch == "arm64"
ArchitecturesAllowed=arm64
ArchitecturesInstallIn64BitMode=arm64
#else
ArchitecturesAllowed=x86compatible
#endif

[Languages]
Name: "english"; MessagesFile: "compiler:Default.isl"

[Tasks]
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"; Flags: unchecked

[Files]
Source: "{#MySourceExe}"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\assets\logo.ico"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\README.md"; DestDir: "{app}\docs"; Flags: ignoreversion
Source: "..\LICENSE"; DestDir: "{app}\docs"; Flags: ignoreversion skipifsourcedoesntexist

[Icons]
Name: "{group}\Lekhani Latex"; Filename: "{app}\lekhani-latex.exe"
Name: "{group}\Documentation"; Filename: "{app}\docs\README.md"
Name: "{group}\Uninstall Lekhani Latex"; Filename: "{uninstallexe}"
Name: "{autodesktop}\Lekhani Latex"; Filename: "{app}\lekhani-latex.exe"; Tasks: desktopicon

[Run]
Filename: "{app}\lekhani-latex.exe"; Description: "{cm:LaunchProgram,Lekhani Latex}"; Flags: nowait postinstall skipifsilent

[UninstallDelete]
Type: filesandordirs; Name: "{app}"
