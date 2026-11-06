# ADR 0001 — Choix de la bibliothèque d'interface

Statut : **proposé**, en attente des mesures sur le PC de tim (AX15).

## Contexte

Le PROMPT (§4) demande un essai avant de choisir entre iced, egui, Slint et Tauri.
Critères, dans l'ordre : (1) mode pixel et transitions de blocs faisables proprement,
(2) 2 000 jeux à 60 images/s, (3) démarrage à froid sous 500 ms, (4) clavier, (5) effort.

Un premier prototype Tauri a été fait le 5 novembre (dossier `launcher` sur le PC).
tim a ensuite précisé qu'il ne veut **que du compilé** : avec Tauri, l'interface est en
HTML/CSS/JavaScript. Tauri et egui ne sont donc pas retenus. Slint n'a pas été essayé
(sa licence demande une lecture attentive pour un projet open source, PROMPT §4).

## Ce que l'essai iced montre

| Critère | Résultat | Preuve |
|---|---|---|
| Page Bibliothèque fidèle à `Mix2` | Oui : en-tête 72 px, liste, filtres, bannière, Jouer, récemment joués | `crates/ui/src/app.rs`, capture `docs/captures/` |
| Logo dessiné en Rust, flamme recolorée | Oui, couleurs identiques aux SVG du design (±2/255) | test `logo::flame_matches_design_files` |
| Transitions de blocs (dissolution, balayage, rideau) | Oui, 16 × 6 blocs, 300 ms | `transition.rs` + tests |
| Mode pixel : police, coins carrés, aplats | Oui, bascule en direct sans redémarrer | bouton « mode » / F3 |
| Mode pixel : vrai rendu basse définition agrandi sans lissage | **Non fait.** iced ne donne pas directement la main sur le rendu de toute la fenêtre ; il faut un passe de post-traitement wgpu (fork léger de `iced_wgpu` ou rendu dans une texture). C'est le risque principal. | — |
| Coins arrondis « en escalier » | **Approché** par un petit rayon ; le vrai escalier demande un rendu dédié | `Mode::radius` |
| Liste virtualisée 2 000 jeux | Oui : seules les lignes visibles sont construites | `sidebar()` |
| Recherche sur 2 000 jeux | 0,2 ms par frappe (filtre simple, la recherche floue arrive au Jalon 1) | mesure |
| Clavier | Flèches, Entrée, champ de recherche ; tabulation entre zones à compléter | — |
| Exe | Un seul fichier de 12 Mo, sans DLL à côté | build `x86_64-pc-windows-gnu` |

## Mesures

Dans le conteneur de développement (pas de carte graphique, rendu logiciel) :
démarrage 225 ms, défilement 34 images/s, transition 25 images/s. **Ces chiffres ne
comptent pas** : sans GPU, tout est dessiné par le processeur. Les vrais chiffres
viendront de `launcher.exe --mesures` sur l'AX15 (fichier `mesures-jalon0.txt`).

## Décision proposée

iced 0.14, avec un passe de post-traitement wgpu pour le mode pixel au Jalon 2.
À confirmer par tim après les mesures sur son PC.

## Conséquences

- Tout est en Rust et compilé.
- Le mode pixel complet demandera du travail bas niveau (shader + rendu dans une texture).
- iced est jeune : certains widgets (liste virtualisée, infobulles accessibles) sont à faire nous-mêmes.
