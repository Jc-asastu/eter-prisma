; ETER PRISMA - instalador Windows (Inno Setup 6)
; Compilar: iscc installer\eter-prisma.iss
; Prerequisito: cargo xtask bundle eter_prisma --release --features webview
; Los bundles se toman de ..\target\bundled\

#define AppName "ETER PRISMA"
#define AppVersion "1.0.0"
#define AppPublisher "Juan Cruz Maisu"
#define AppURL "https://jcmaisu.tech"

[Setup]
AppId={{7E3A9C41-5D2B-4F8E-9A1C-3B6D0E52A7F4}
AppName={#AppName}
AppVersion={#AppVersion}
AppPublisher={#AppPublisher}
AppPublisherURL={#AppURL}
AppSupportURL={#AppURL}
DefaultDirName={autopf64}\ETER\PRISMA
DisableProgramGroupPage=yes
LicenseFile=..\LICENSE
OutputDir=output
OutputBaseFilename=eter-prisma-{#AppVersion}-win64-setup
Compression=lzma2/max
SolidCompression=yes
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
WizardStyle=modern
UninstallDisplayName={#AppName} {#AppVersion}

[Types]
Name: "full"; Description: "VST3 + CLAP (recomendado)"
Name: "custom"; Description: "Eleccion manual"; Flags: iscustom

[Components]
Name: "vst3"; Description: "Plugin VST3"; Types: full custom
Name: "clap"; Description: "Plugin CLAP"; Types: full custom

[Files]
; VST3: bundle-dir completo al folder estandar del sistema
Source: "..\target\bundled\eter_prisma.vst3\*"; DestDir: "{commoncf64}\VST3\ETER PRISMA.vst3"; Components: vst3; Flags: ignoreversion recursesubdirs createallsubdirs
; CLAP: archivo unico al folder estandar
Source: "..\target\bundled\eter_prisma.clap"; DestDir: "{commoncf64}\CLAP"; DestName: "ETER PRISMA.clap"; Components: clap; Flags: ignoreversion
; licencia como referencia en el dir de instalacion
Source: "..\LICENSE"; DestDir: "{app}"; Flags: ignoreversion

[Messages]
SetupWindowTitle=Instalar {#AppName} {#AppVersion}
