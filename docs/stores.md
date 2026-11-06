# Boutiques : chemins et protocoles

À vérifier sur une vraie machine (PROMPT §7). « Testé » = couvert par une fixture, pas encore vérifié sur un PC réel.

| Boutique | Jeux installés | Lancement | État |
|---|---|---|---|
| Steam | registre `HKCU\Software\Valve\Steam\SteamPath`, `steamapps/libraryfolders.vdf`, `appmanifest_*.acf` ; temps de jeu dans `userdata/*/config/localconfig.vdf` ; images dans `appcache/librarycache` | `steam://rungameid/<appid>` | Testé (fixtures) |
| Epic | `%PROGRAMDATA%\Epic\EpicGamesLauncher\Data\Manifests\*.item` ; DLC et installs incomplètes ignorés | `com.epicgames.launcher://apps/<namespace>%3A<itemId>%3A<appName>?action=launch&silent=true` (le PROMPT proposait `apps/<AppName>` : la forme longue est celle qu'utilise Epic, à confirmer) | Testé (fixtures) |
| GOG | registre `HKLM\SOFTWARE\WOW6432Node\GOG.com\Games\<id>` ; Galaxy dans `...\GalaxyClient\paths\client` | `GalaxyClient.exe /command=runGame /gameId=<id> /path=<dossier>`, sinon l'exe du jeu | Testé (logique), registre non testé |
