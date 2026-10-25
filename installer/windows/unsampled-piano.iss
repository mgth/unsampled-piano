; Windows installer (Inno Setup 6): the standalone player and the plugins,
; each one chosen by the user, and the presets they share. Installs for all
; users (Program Files, Common Files\VST3 and \CLAP, ProgramData) or,
; without administrator rights, for the current user only
; (%LOCALAPPDATA%\Programs, ...\Common\VST3 and \CLAP, %LOCALAPPDATA%).
;
; Expects the built files (see the Standalone workflow):
;   target\dist\unsampled-piano.exe
;   target\bundled\Unsampled Piano.clap, Unsampled Piano.vst3\
; iscc /DAppVersion=0.2.1 installer\windows\unsampled-piano.iss
;   -> target\installer\unsampled-piano-setup-windows-x86_64.exe

#ifndef AppVersion
  #define AppVersion "0.0.0"
#endif
#define Root "..\.."
#define AppName "Unsampled Piano"

[Setup]
; Never change: Windows identifies the installed application by it.
AppId={{B92DA9EA-BF84-41D8-B818-725AC6B03FD4}
AppName={#AppName}
AppVersion={#AppVersion}
AppVerName={#AppName} {#AppVersion}
AppPublisher=mgth
AppPublisherURL=https://github.com/mgth/unsampled-piano
AppSupportURL=https://github.com/mgth/unsampled-piano/issues
DefaultDirName={autopf}\{#AppName}
DefaultGroupName={#AppName}
DisableProgramGroupPage=yes
LicenseFile={#Root}\LICENSE
; All users by default; the dialog offers the current user only.
PrivilegesRequired=admin
PrivilegesRequiredOverridesAllowed=dialog
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
MinVersion=10.0
OutputDir={#Root}\target\installer
OutputBaseFilename=unsampled-piano-setup-windows-x86_64
SetupIconFile={#Root}\assets\icon.ico
; The executable carries the same icon (crates/piano-play/build.rs).
UninstallDisplayIcon={app}\unsampled-piano.exe
UninstallDisplayName={#AppName}
Compression=lzma2/max
SolidCompression=yes
WizardStyle=modern

[Languages]
Name: "en"; MessagesFile: "compiler:Default.isl"
Name: "fr"; MessagesFile: "compiler:Languages\French.isl"

[CustomMessages]
en.TypeFull=Standalone application and plugins
en.TypeStandalone=Standalone application only
en.TypePlugins=Plugins only (VST3 and CLAP)
en.TypeCustom=Custom installation
en.CompStandalone=Standalone application (WASAPI and ASIO output, MIDI keyboard)
en.CompVst3=VST3 plugin
en.CompClap=CLAP plugin
en.DesktopIcon=Create a desktop shortcut
en.Launch=Start Unsampled Piano
fr.TypeFull=Application autonome et plugins
fr.TypeStandalone=Application autonome seule
fr.TypePlugins=Plugins seuls (VST3 et CLAP)
fr.TypeCustom=Installation personnalisée
fr.CompStandalone=Application autonome (sortie WASAPI et ASIO, clavier MIDI)
fr.CompVst3=Plugin VST3
fr.CompClap=Plugin CLAP
fr.DesktopIcon=Créer un raccourci sur le bureau
fr.Launch=Lancer Unsampled Piano

[Types]
Name: "full"; Description: "{cm:TypeFull}"
Name: "standalone"; Description: "{cm:TypeStandalone}"
Name: "plugins"; Description: "{cm:TypePlugins}"
Name: "custom"; Description: "{cm:TypeCustom}"; Flags: iscustom

[Components]
Name: "standalone"; Description: "{cm:CompStandalone}"; Types: full standalone
Name: "vst3"; Description: "{cm:CompVst3}"; Types: full plugins
Name: "clap"; Description: "{cm:CompClap}"; Types: full plugins

[Tasks]
Name: "desktopicon"; Description: "{cm:DesktopIcon}"; Components: standalone; Flags: unchecked

[Files]
Source: "{#Root}\LICENSE"; DestDir: "{app}"; DestName: "LICENSE.txt"
Source: "{#Root}\target\dist\unsampled-piano.exe"; DestDir: "{app}"; Components: standalone; Flags: ignoreversion
; The presets, for the application and the plugins alike: where they look
; for installed presets (piano_host::installed_preset_dirs). The user's own
; are saved in %APPDATA%\unsampled-piano\presets and stay on uninstall.
Source: "{#Root}\presets\*"; DestDir: "{commonappdata}\{#AppName}\presets"; Check: IsAdminInstallMode; Flags: ignoreversion recursesubdirs createallsubdirs
Source: "{#Root}\presets\*"; DestDir: "{localappdata}\{#AppName}\presets"; Check: not IsAdminInstallMode; Flags: ignoreversion recursesubdirs createallsubdirs
; {autocf}: Common Files for all users, %LOCALAPPDATA%\Programs\Common for
; the current user: where hosts look for VST3 and CLAP plugins.
Source: "{#Root}\target\bundled\{#AppName}.vst3\*"; DestDir: "{autocf}\VST3\{#AppName}.vst3"; Components: vst3; Flags: ignoreversion recursesubdirs createallsubdirs
Source: "{#Root}\target\bundled\{#AppName}.clap"; DestDir: "{autocf}\CLAP"; Components: clap; Flags: ignoreversion

[Icons]
Name: "{autoprograms}\{#AppName}"; Filename: "{app}\unsampled-piano.exe"; Components: standalone
Name: "{autodesktop}\{#AppName}"; Filename: "{app}\unsampled-piano.exe"; Tasks: desktopicon

[Run]
Filename: "{app}\unsampled-piano.exe"; Description: "{cm:Launch}"; Components: standalone; Flags: nowait postinstall skipifsilent
