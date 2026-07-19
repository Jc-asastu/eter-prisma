; ETER PRISMA - instalador Windows (Inno Setup 6)
; Compilar: iscc installer\eter-prisma.iss
; Prerequisito: cargo xtask bundle eter_prisma --release --features webview
; Los bundles se toman de ..\target\bundled\
; Estetica: banco optico (banner + onda espectral como barra de progreso).
; /VERYSILENT sigue soportado (el codigo visual se salta en modo silencioso).

#define AppName "ETER PRISMA"
#define AppVersion "1.0.2"
#define AppPublisher "Asastu"
#define AppURL "https://github.com/Jc-asastu/eter-prisma"

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
WizardSizePercent=110
WizardImageFile=assets\wizard-banner.bmp
WizardSmallImageFile=assets\wizard-small.bmp
UninstallDisplayName={#AppName} {#AppVersion}

[Types]
Name: "full"; Description: "VST3 + CLAP (recommended)"
Name: "custom"; Description: "Manual selection"; Flags: iscustom

[Components]
Name: "vst3"; Description: "VST3 plugin"; Types: full custom
Name: "clap"; Description: "CLAP plugin"; Types: full custom

[Files]
; VST3: bundle-dir completo al folder estandar del sistema
Source: "..\target\bundled\eter_prisma.vst3\*"; DestDir: "{commoncf64}\VST3\ETER PRISMA.vst3"; Components: vst3; Flags: ignoreversion recursesubdirs createallsubdirs
; CLAP: archivo unico al folder estandar
Source: "..\target\bundled\eter_prisma.clap"; DestDir: "{commoncf64}\CLAP"; DestName: "ETER PRISMA.clap"; Components: clap; Flags: ignoreversion
; licencia como referencia en el dir de instalacion
Source: "..\LICENSE"; DestDir: "{app}"; Flags: ignoreversion

[Messages]
SetupWindowTitle=ETER PRISMA {#AppVersion} — optical bench installer
WelcomeLabel1=PRISMA%nspectral dispersion
WelcomeLabel2=Inside glass, every wavelength travels at its own speed. That is why a prism fans white light into a rainbow.%n%nNewton did it to sunlight in 1666. PRISMA does it to your sound: every frequency arrives with its own delay, following a curve you draw. Pure phase, zero coloration.%n%nThis will install PRISMA {#AppVersion} (VST3 + CLAP) on your machine.
ClickNext=Step into the laboratory.
WizardLicense=The fine print
LicenseLabel3=PRISMA is free software under GPL-3.0. Read the light, share the light. Accept to continue.
WizardSelectComponents=Choose your optics
SelectComponentsDesc=Which plugin formats should land on this machine?
WizardReady=Ready to refract
ReadyLabel1=The bench is set.
ReadyLabel2a=Click Install to let the light through. The wave below will show the dispersion in progress.
WizardInstalling=Refracting
InstallingLabel=PRISMA is being dispersed into your system. Each frequency arrives in its own time — so does each file.
FinishedHeadingLabel=The spectrum is yours
FinishedLabelNoIcons=PRISMA is installed. Open your DAW, load ETER PRISMA, feed it a drum bus and turn Spread.%n%nDrums in, rainbows out.%n%nIf Windows SmartScreen frowned at the installer: that is what happens to unsigned free software made with love instead of certificates.%n%ncreated with ♥ by Asastu — github.com/Jc-asastu/eter-prisma
ClickFinish=Leave the laboratory.

[Code]
{ ═══ onda espectral como barra de progreso ═══
  Oculta la ProgressGauge y dibuja una forma de onda coloreada por longitud
  de onda; el progreso ilumina la onda de izquierda a derecha.
  En modo silencioso no se toca nada (WizardSilent). }

var
  WaveImg: TBitmapImage;
  WaveTimer: LongWord;

function SetTimer(hWnd: LongWord; nIDEvent, uElapse: LongWord;
  lpTimerFunc: LongWord): LongWord;
  external 'SetTimer@user32.dll stdcall';
function KillTimer(hWnd: LongWord; nIDEvent: LongWord): BOOL;
  external 'KillTimer@user32.dll stdcall';
function GetTickCount: LongWord;
  external 'GetTickCount@kernel32.dll stdcall';

{ color espectral 0..1 -> TColor (BGR) }
function WlColor(T: Double): TColor;
var R, G, B: Integer; Seg: Double; I: Integer;
begin
  if T < 0 then T := 0; if T > 1 then T := 1;
  Seg := T * 6.0; I := Trunc(Seg); Seg := Seg - I;
  case I of
    0: begin R:=255+Trunc((255-255)*Seg); G:=59+Trunc((149-59)*Seg);  B:=48+Trunc((0-48)*Seg);   end;
    1: begin R:=255;                      G:=149+Trunc((214-149)*Seg);B:=0+Trunc((10-0)*Seg);    end;
    2: begin R:=255+Trunc((48-255)*Seg);  G:=214+Trunc((209-214)*Seg);B:=10+Trunc((88-10)*Seg);  end;
    3: begin R:=48+Trunc((64-48)*Seg);    G:=209+Trunc((200-209)*Seg);B:=88+Trunc((224-88)*Seg); end;
    4: begin R:=64+Trunc((58-64)*Seg);    G:=200+Trunc((123-200)*Seg);B:=224+Trunc((255-224)*Seg);end;
  else begin R:=58+Trunc((176-58)*Seg);   G:=123+Trunc((76-123)*Seg); B:=255;                    end;
  end;
  Result := (B shl 16) or (G shl 8) or R;
end;

procedure DrawWave;
var
  Bmp: TBitmap;
  W, H, X, Mid: Integer;
  Frac, Ph, Amp, Y: Double;
  Lit: Integer;
  C: TColor;
  R: TRect;
begin
  if WaveImg = nil then Exit;
  W := WaveImg.Width; H := WaveImg.Height;
  if (W <= 0) or (H <= 0) then Exit;
  Bmp := TBitmap.Create;
  try
    Bmp.Width := W; Bmp.Height := H;
    Bmp.Canvas.Brush.Color := $0A0906;           { #05060a aprox en BGR }
    R.Left := 0; R.Top := 0; R.Right := W; R.Bottom := H;
    Bmp.Canvas.FillRect(R);
    Mid := H div 2;
    if WizardForm.ProgressGauge.Max > 0 then
      Frac := WizardForm.ProgressGauge.Position / WizardForm.ProgressGauge.Max
    else
      Frac := 0;
    Lit := Trunc(Frac * W);
    Ph := GetTickCount / 240.0;                  { fase animada }
    for X := 0 to W - 1 do
    begin
      { onda: dos senos superpuestos, amplitud respirando }
      Amp := (H * 0.30) * (0.62 + 0.38 * Sin(X / 34.0 + Ph * 0.7));
      Y := Sin(X / 9.5 + Ph) * Amp + Sin(X / 3.7 - Ph * 1.6) * Amp * 0.22;
      if X <= Lit then
        C := WlColor(X / W)                      { iluminada: color espectral }
      else
        C := $2A2620;                            { apagada: gris azulado }
      Bmp.Canvas.Pen.Color := C;
      Bmp.Canvas.MoveTo(X, Mid);
      Bmp.Canvas.LineTo(X, Mid - Round(Y));
      { reflejo tenue }
      if X <= Lit then
      begin
        Bmp.Canvas.Pen.Color := (C and $7F7F7F);
        Bmp.Canvas.MoveTo(X, Mid);
        Bmp.Canvas.LineTo(X, Mid + Round(Y * 0.35));
      end;
    end;
    { cabezal: columna blanca en el frente del progreso }
    if (Lit > 0) and (Lit < W) then
    begin
      Bmp.Canvas.Pen.Color := $F0F6FF;
      Bmp.Canvas.MoveTo(Lit, Mid - Round(H * 0.42));
      Bmp.Canvas.LineTo(Lit, Mid + Round(H * 0.28));
    end;
    WaveImg.Bitmap := Bmp;
  finally
    Bmp.Free;
  end;
end;

procedure WaveTick(H: LongWord; Msg: LongWord; IdEvent: LongWord; Time: LongWord);
begin
  DrawWave;
end;

procedure InitializeWizard;
begin
  if WizardSilent then Exit;
  { la onda vive en la pagina de instalacion, sobre la gauge oculta }
  WaveImg := TBitmapImage.Create(WizardForm);
  WaveImg.Parent := WizardForm.ProgressGauge.Parent;
  WaveImg.Left := WizardForm.ProgressGauge.Left;
  WaveImg.Top := WizardForm.ProgressGauge.Top - ScaleY(14);
  WaveImg.Width := WizardForm.ProgressGauge.Width;
  WaveImg.Height := WizardForm.ProgressGauge.Height + ScaleY(34);
  WaveImg.BackColor := $0A0906;
  WaveImg.Visible := False;
  WizardForm.ProgressGauge.Visible := False;
end;

procedure CurPageChanged(CurPageID: Integer);
begin
  if WizardSilent then Exit;
  if CurPageID = wpInstalling then
  begin
    WaveImg.Visible := True;
    DrawWave;
    if WaveTimer = 0 then
      WaveTimer := SetTimer(0, 0, 33, CreateCallback(@WaveTick));
  end
  else if WaveTimer <> 0 then
  begin
    KillTimer(0, WaveTimer);
    WaveTimer := 0;
    if WaveImg <> nil then WaveImg.Visible := False;
  end;
end;

procedure DeinitializeSetup;
begin
  if WaveTimer <> 0 then
  begin
    KillTimer(0, WaveTimer);
    WaveTimer := 0;
  end;
end;
