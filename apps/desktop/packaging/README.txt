VUA 0.6.6 Ibis (Beta) — Windows x64

Extract the entire ZIP, then run VUA.exe. Keep resources, locales and all companion
files together. Node.js, pnpm, Rust and the source repository are not needed.

This current-state release is unsigned. Windows may warn about an unfamiliar
publisher. AMF is bundled and enabled for new users; saved choices are retained.
Steam, SteamVR, VRChat, PICO Connect, Unity and VRCFT are obtained separately.
Use Play/Tools/Avatar editing and Help for their official setup and account steps.
Physical desktop/PICO, missing-software installation and AMF material acceptance
remain incomplete; read the release notes before relying on an untested path:
https://github.com/VUA-Project/VUA/releases/tag/v0.6.6

Data is stored separately in %APPDATA%\VUA. To update, exit VUA and extract the
new ZIP into another folder. The new copy uses the same data directory. Back up
Beta data first. Remove the extracted program folder to uninstall; retained data
can be removed separately when no longer needed. No NSIS or incremental updater
is included in this release; these are subsequent work.

resources\notices contains VUA LICENSE/NOTICE, upstream license texts and the
exact-build dependency inventory under build/. Electron/Chromium notices accompany
the runtime; MPL source packages are supplied in build/sources/.
