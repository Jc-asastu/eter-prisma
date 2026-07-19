; QA VARIANT — solo para smoke test silencioso local. NO SE SHIPPEA.
; Igual al eter-prisma.iss en lo relevante pero: lowest privileges +
; DestDir en LocalAppData + sin uninstaller. Valida el guard WizardSilent.

#define AppName "ETER PRISMA QA"
#define AppVersion "1.0.2"

[Setup]
AppId={{9F1B2C33-0A00-4000-8000-00000000A0A1}
AppName={#AppName}
AppVersion={#AppVersion}
PrivilegesRequired=lowest
DefaultDirName={localappdata}\eter-prisma-qa
DisableProgramGroupPage=yes
LicenseFile=..\LICENSE
OutputDir=output
OutputBaseFilename=qa-preview-setup
Compression=lzma2/fast
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
WizardStyle=modern
WizardSizePercent=110
WizardImageFile=assets\wizard-banner.bmp
WizardSmallImageFile=assets\wizard-small.bmp
CreateUninstallRegKey=no
Uninstallable=no

[Files]
Source: "..\target\bundled\eter_prisma.vst3\*"; DestDir: "{localappdata}\eter-prisma-qa\VST3"; Flags: ignoreversion recursesubdirs createallsubdirs
Source: "..\target\bundled\eter_prisma.clap"; DestDir: "{localappdata}\eter-prisma-qa\CLAP"; DestName: "ETER PRISMA.clap"; Flags: ignoreversion

[Code]
{ mismo esqueleto de codigo que el installer real — valida el guard WizardSilent }
var
  WaveImg: TBitmapImage;
  WaveTimer: LongWord;

function SetTimer(hWnd: LongWord; nIDEvent, uElapse: LongWord;
  lpTimerFunc: LongWord): LongWord;
  external 'SetTimer@user32.dll stdcall';
function KillTimer(hWnd: LongWord; nIDEvent: LongWord): BOOL;
  external 'KillTimer@user32.dll stdcall';

procedure WaveTick(H: LongWord; Msg: LongWord; IdEvent: LongWord; Time: LongWord);
begin
end;

procedure InitializeWizard;
begin
  if WizardSilent then Exit;
  WaveImg := TBitmapImage.Create(WizardForm);
  WaveImg.Parent := WizardForm.ProgressGauge.Parent;
  WaveImg.Visible := False;
  WizardForm.ProgressGauge.Visible := False;
end;

procedure CurPageChanged(CurPageID: Integer);
begin
  if WizardSilent then Exit;
  if CurPageID = wpInstalling then
  begin
    if WaveTimer = 0 then
      WaveTimer := SetTimer(0, 0, 33, CreateCallback(@WaveTick));
  end
  else if WaveTimer <> 0 then
  begin
    KillTimer(0, WaveTimer);
    WaveTimer := 0;
  end;
end;
