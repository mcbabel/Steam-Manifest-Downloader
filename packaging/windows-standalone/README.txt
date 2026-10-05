Steam Manifest Downloader – Standalone version
==============================================

This version needs no installation and no administrator rights.
Everything the emulator step needs is already included.


How to start
------------
1. Extract the whole ZIP file into its own folder, for example
   "Documents\Steam Manifest Downloader".
   Important: do not run it from inside the ZIP and do not extract it to
   "C:\Program Files".
2. Double-click "Steam Manifest Downloader.exe".
3. If Windows shows "Windows protected your PC": click "More info", then
   "Run anyway".

Keep the "third-party" folder next to the .exe. It contains the emulator
(gbe_fork), Steamless and the Steam API bypass.


What you do NOT need
--------------------
- No .NET runtime. The built-in downloader works without it. .NET 9 is only
  needed if you switch to the "DepotDownloaderMod" engine in Settings.
- No extra downloads for the emulator.


Tips if something does not work
-------------------------------
- Adding a game to your Steam library: Steam has to be closed while it is
  added. If Steam is running, the app offers to close it and start it again.
- Steam is installed somewhere else and the app does not find it: click
  "Choose Steam folder" next to "Also add to Steam library", or set it under
  Settings -> Steam folder. Pick the folder that contains steam.exe. No
  administrator rights are needed for this.
- Emulator: download games into a folder you own (e.g. Documents or the
  Desktop), not into "C:\Program Files". Nothing can be changed there
  without administrator rights.
- Antivirus: Windows Defender sometimes wrongly reports emulator files as a
  threat and deletes them. You can allow them again under "Windows Security
  -> Virus & threat protection -> Protection history". This can need
  administrator rights. If you are not allowed to change the antivirus on
  this PC, the emulator cannot be used here.
- Microsoft's WebView2 runtime is normally already installed on Windows 10
  and 11. If the app does not start at all, it is probably missing. Help and
  bug reports: https://github.com/mcbabel/Steam-Manifest-Downloader/issues

Licenses of the included programs: see THIRD-PARTY-NOTICES.txt
