[Setup]
AppName=Lekhani Latex
AppVersion=0.1.0
AppPublisher=Borneel Bikash Phukan
AppPublisherURL=https://github.com/example/lekhani-latex
DefaultDirName={autopf}\lekhani-latex
DefaultGroupName=Lekhani Latex
OutputBaseFilename=lekhani-latexSetup
Compression=lzma2
SolidCompression=yes
ArchitecturesInstallIn64BitMode=x64
OutputDir=Output
SetupIconFile=assets\logo.ico

[Files]
Source: "target\release\lekhani-latex.exe"; DestDir: "{app}"; Flags: ignoreversion

[Icons]
Name: "{group}\Lekhani Latex"; Filename: "{app}\lekhani-latex.exe"
Name: "{autodesktop}\Lekhani Latex"; Filename: "{app}\lekhani-latex.exe"; Tasks: desktopicon

[Tasks]
Name: "desktopicon"; Description: "Create a desktop shortcut"; GroupDescription: "Additional icons:"

[Run]
Filename: "{app}\lekhani-latex.exe"; Description: "Launch Lekhani Latex"; Flags: nowait postinstall skipifsilent
