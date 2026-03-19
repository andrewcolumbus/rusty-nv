[Setup]
AppName=rust-nv
AppVersion=0.1.0
AppPublisher=Antic Studios
AppPublisherURL=https://github.com/acswi/rust-nv
DefaultDirName={autopf}\rust-nv
DefaultGroupName=rust-nv
SetupIconFile=logo.ico
UninstallDisplayIcon={app}\rust-nv.exe
OutputDir=installer
OutputBaseFilename=rust-nv-0.1.0-setup
Compression=lzma2
SolidCompression=yes
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
WizardStyle=modern
PrivilegesRequired=lowest
PrivilegesRequiredOverridesAllowed=dialog

[Files]
Source: "target\release\rust-nv.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "README.md"; DestDir: "{app}"; Flags: ignoreversion
Source: "logo.ico"; DestDir: "{app}"; Flags: ignoreversion
Source: "logo.png"; DestDir: "{app}"; Flags: ignoreversion
Source: "LICENSE"; DestDir: "{app}"; Flags: ignoreversion skipifsourcedoesntexist

[Icons]
Name: "{group}\rust-nv"; Filename: "{app}\rust-nv.exe"; IconFilename: "{app}\logo.ico"
Name: "{autodesktop}\rust-nv"; Filename: "{app}\rust-nv.exe"; IconFilename: "{app}\logo.ico"; Tasks: desktopicon

[Tasks]
Name: "desktopicon"; Description: "Create a &desktop shortcut"; GroupDescription: "Additional icons:"

[Run]
Filename: "{app}\rust-nv.exe"; Description: "Launch rust-nv"; Flags: nowait postinstall skipifsilent
