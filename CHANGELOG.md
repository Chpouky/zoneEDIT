# Journal des modifications

## 0.9.0

### Fonctionnalités

- **Spawners d'artefacts du monde** : nouvelle section **ARTIFACTS** du
  panneau latéral et catégorie **Spawners d'artefacts** — le fichier
  `ArtifactSpawnerPrototypes.cfg` (93 spawners) est chargé : **une ligne par
  spawner × rang** (Newbie → Master, 364 lignes en vanilla) avec 10 colonnes —
  **Count**, **Radius**, **Min/MaxCooldown**, **chance de spawn** (base/bonus)
  et les 4 **chances de rareté** (Common/Uncommon/Rare/Epic, sous-scope
  `RarityChance` du cfg). Les valeurs de rareté sont des fractions 0..1 comme
  dans le fichier du jeu. Chaque ligne affiche l'icône du **premier artefact**
  (non vide) de la liste `ListOfArtifacts` du spawner (le SID du spawner n'a
  pas d'icône propre ; les spawners dont le premier artefact est absent ou
  sans fichier d'icône — 8 en vanilla — gardent l'icône générique).
- **Alias de clés cfg** : les clés fautes d'orthographe du jeu — `weight`
  (minuscule, 152 lignes dans `ItemGeneratorPrototypes.cfg`) et
  `AmmoMaxcount` (4 lignes dans les loadouts NPC) — sont maintenant lues
  comme **Weight** / **AmmoMaxCount** et éditables — nouvelle table
  `[parser.aliases]` dans la config (alias → clé canonique, vérifiée au
  chargement). L'export garde la clé d'origine, telle qu'écrite dans le
  fichier.
- **Durabilité des armes** : nouveaux champs **MinDurability** / **MaxDurability**
  (fraction 0..1 de la durabilité max) éditables sur les lignes d'armes —
  catégories **Weapons Loaded Ammo** et **NPC loadouts**. Cohérence
  min ≤ max vérifiée à la validation (comme MinCount/MaxCount).

### Interface

- **En-têtes de colonnes sans chevauchement** : les étiquettes des 9
  nouvelles colonnes de spawners d'artefacts sont raccourcies pour tenir
  dans la grille de colonnes de ~70 px (« Min cool », « Base », « Bonus »,
  « Common »… ; le nom complet reste dans la modale d'édition). Sur les pages
  de stashes, « Min Spawn % » / « Max Spawn % » deviennent « **Min %** » /
  « **Max %** » (à côté de « Min Count » / « Max Count ») — l'ancien
  « Max Spawn % » débordait déjà de quelques pixels sur sa voisine. Garde-fou
  dans la peinture de l'en-tête : une étiquette plus large que la pas de
  colonne est tronquée avec « … » (`fit_label`, déjà utilisé pour les noms de
  lignes) plutôt que laissée en chevauchement.

### Correction

- **Export (clés d'alias)** : les patches réémettaient la **clé canonique** du
  champ (`Weight`, `ArtifactCount`…) au lieu de la **clé du fichier de base**
  (`weight`, `Count`, `Common`…). Le merge de ZoneKit aurait alors **ajouté une
  nouvelle clé** au lieu de remplacer la valeur — l'édition n'aurait eu aucun
  effet dans le jeu. Le champ porte désormais la clé telle qu'écrite dans le
  fichier ; le patch (et le rendu complet, déjà correct) la réémet à
  l'identique. Test d'export dédié.
- **Export** : le mot-clé `{bpatch}` n'est plus émis que sur le **premier
  nœud** (root) de chaque patch — le merge étant récursif, les scopes
  imbriqués sont des `struct.begin` simples. L'exporter le posait sur chaque
  `struct.begin` (redondant). Tests d'export mis à jour ; doc de référence
  (skill `stalker-cfg-patching`) corrigée en conséquence.

## 0.8.0

### Données : section STATS « Anomalies »

- Nouvelle section **STATS** et catégorie **Anomalies** : le fichier
  `AnomalyPrototypes.cfg` est chargé tel quel, **un bloc = une entrée**.
- 11 champs de comportement éditables par anomalie : **Radius**, **Cooldown**,
  **Duration**, **ActivationDelay**, **ActivationSpeedThreshold**,
  **DefaultHeightOffset**, **CollisionHeight**, **MovementLocationPriorityRadius**,
  **MovementSpeed**, **FakeArtifactSpawnAmountMin** / **FakeArtifactSpawnAmountMax**.
- **Hiérarchie** anomalie (parent + variantes `_Dynamic`/enfants) : les enfants
  partagent la **teinte** de leur parent et portent un préfixe **« ↳ »** collé au
  nom (même colonne, sans icône), sur le modèle des lignes de délégation
  (Medical Crates).
- Nouveau mécanisme de config **`block_entry_roles`** : pour ces rôles de
  fichier, le bloc de niveau 1 est lui-même l'unité éditée (pas de
  `ItemPrototypeSID`) — les champs sont les clés plates sous le bloc.

### Interface

- **Refonte de la couche de lignes** : géométrie par type de ligne
  (`RowLayout`) + chrome commun (`row_frame`) + toutes les constantes dans
  `metrics.rs`. Item, générateur, rang, set, orphelin et anomalie passent tous
  par le même moule — plus de peinture de ligne codée en dur.
- Taille de fenêtre par défaut augmentée (1400×720 → **1500×820**).

### Divers

- Commentaires legacy français traduits en anglais.
- Tests : rendu de la hiérarchie d'anomalies + parsing (fixture
  `AnomalyPrototypes.cfg`).

## 0.7.5

### Interface

- Sidebar : le label « ITEM TYPES » et le champ de recherche sont remplacés
  par un **dropdown de section**. Les sections sont pilotées par la config
  (champ `section` de `[[categories]]`, défaut `ITEM SPAWNS`) : chaque
  section regroupe ses catégories, et la liste s'adapte à la sélection.
- Le **champ de recherche** est déplacé dans la **barre de filtres** en
  haut (à droite), à côté des filtres de fichier et de références ; il
  fonctionne avec l'ensemble des filtres.

## 0.7.4

### Export (refonte complète)

- L'export génère des **patches ZoneKit conformes** : un `<Mod>.cfg` par
  fichier modifié, placé dans le sous-dossier prototype qui miroite le
  chemin de la base (`GameData/X.cfg` → `GameData/X/<Mod>.cfg`) — plus de
  miroir complet des fichiers.
- Chaque bloc modifié est patché avec le mot-clé **`{bpatch}`** : seuls
  les champs modifiés sont écrits, **à leur profondeur réelle** (la chaîne
  de scopes entre le bloc et le champ est reconstruite en nœuds `{bpatch}`
  imbriqués) ; les autres champs du fichier de base sont intacts.
- Le nœud est ciblé par son nom tel qu'écrit dans la base ; les
  attributs de référence (`{refkey=…}`, `{refurl=…}`) ne sont plus
  ré-émis, et plus de commentaire d'en-tête dans le fichier exporté.
- Aucun nettoyage agressif : zoneEDIT n'écrit (réécrit) que ses propres
  `<Mod>.cfg` — les autres fichiers du mod ne sont jamais touchés.
- Correction de l'arborescence : la racine de contenu est
  `<mod>/Content/GameLite` (les chemins de base contiennent déjà
  `GameData/`) — plus de double `GameData/GameData` ; une structure
  existante (source ZoneKit, `GameData` en racine, ou le dossier
  GameData lui-même) est détectée plutôt que recréée.

### Interface

- Le champ « Nom du mod » est supprimé : le bouton **Export** ouvre
  l'explorateur de dossiers et le nom du mod est le **nom du dossier
  choisi**.
- Le dernier dossier choisi est mémorisé (`~/.config/zoneedit/state.toml`)
  et rechargé automatiquement au démarrage ; l'explorateur ne s'ouvre que
  s'il n'y en a pas encore un.
- Barre de statut : indicateur « struct.begin / struct.end balanced »
  supprimé.

## 0.7.3

### Divers

- Traduction de l'intégralité des commentaires (headers de module, doc
  comments, commentaires inline, messages de panics/tests) du français à
  l'anglais — convention : toute la codebase est commentée en anglais.
  Aucun changement de comportement.

## 0.7.2

### Filtres

- Barre de filtres renommée **« Filters »**, filtre mono-sélection abandonné
  au profit de **filtres cumulables** (union) : on peut combiner plusieurs
  rangs, « No Rank » et « Trade » librement.
- Chip **« All » supprimé** : aucun filtre actif = tout afficher ; un ou
  plusieurs filtres actifs = union des correspondances.
- Nouveau filtre **« Trade »** : ne montre que les entrées contenant « trade »
  (SID, bloc ou contexte, insensible à la casse) — inventaires de traders.

### Barre batch

- Bouton « Reset » renommé **« Reset Selected »** : ne remet plus à zéro que
  les rows sélectionnées (aucun effet sans sélection).
- Nouveau bouton **« Reset All »** : remet à zéro toutes les valeurs du
  workspace.

### Diff preview

- Pop-up refaite : les modifications sont **groupées par item** (une ligne
  par item, icône + nom + valeurs changées), regroupées par fichier puis par
  bloc, au lieu d'une ligne par champ.
- Plus de flèche de repli sur la fenêtre.

### Divers

- Icônes des jumelles (Binoculars_01/02/03).
- Temps de frame affiché dans la barre de statut.
- Nettoyage clippy (`-D warnings` vert) : modules de tests en fin de
  fichiers, docs et formats en ligne.
- Nouveau `AGENTS.md` : guide d'architecture et de conventions pour les
  agents de codage.

## 0.7.1

### Icônes

- Icônes des **armes** issues du nouvel export UI (variantes `_body`,
  non-upgrade), joueurs et gardes ; 20 armes de l'ancien export remappées
  (AK74 + variante Strelok, AKU, Silence SMG, M16, Obrez, PM + SkifGun,
  TOZ, Viper, G37 + V2…).
- Suppression des 80 `Blueprint_*.png` redondants (identiques à la
  repli `_blueprint.png`) : le binaire est plus léger.
- `build.rs` : `rerun-if-changed` par fichier PNG (le watch de dossier
  ignorait les icônes ajoutées).

### Divers

- Libellé de rang toujours visible (même quand un filtre de rang est actif).

## 0.7.0

### Filtre de difficulté

- Les rangs codés dans le **nom de bloc** sont maintenant pris en compte
  (suffixe `_Rank` et CamelCase, ex. `Master`), en plus du champ `Rank` /
  `PlayerRank` qui reste prioritaire.
- Quand un rang est sélectionné, les entrées **sans rang** sont masquées.
- Nouveau filtre **« No Rank »** : ne montre que les entrées sans rang.

### Classification

- Les détecteurs d'anomalies (**Bear, Gilka, Veles, Echo, Oko**) sortent de
  Loot/Artifacts/Accessoires pour une source de spawn dédiée
  **« Detectors »** (Équipement).

### Nettoyage

- Filtre de **type supprimé** définitivement (UI, état, config, logique).
  Les anciennes configs contenant `[[type_filters]]` sont rejetées
  explicitement.

### Icônes

- Les items dont le SID contient « blueprint » (insensible à la casse)
  affichent l'icône `_blueprint.png` au lieu du placeholder.

### Champ de recherche

- Bouton **« × »** d'effacement à droite du champ : grisé et inactif quand le
  champ est vide, normal sinon (mêmes états que les boutons batch).
- Texte du champ centré verticalement par le layout (plus de positionnement
  absolu).
- La croix est **dessinée par le Painter** (deux segments diagonaux au centre
  exact du bouton) : le glyphe « × » de la police n'est pas centré dans sa
  box de ligne (encre mesurée +1,5 px sous le centre), le dessin géométrique
  garantit un centrage exact, indépendant de la police.

### Tests

- 70/70 (46 lib + 24 intégration) ; tests UI headless adaptés
  (filtre de rang, bouton « × », croix en deux `LineSegment`).

## 0.6.0

- Icônes par item (260 PNG des exports du jeu, nommés par SID), embarquées
  à la compilation (`build.rs`) et décodées paresseusement via `egui_extras`.
- Fit vertical des icônes (ratio plafonné à 2.0, réf. M16) ; noms des items
  alignés sur une colonne fixe, quelle que soit l'icône.
- Pop-up au survol d'une icône : image en taille originale sur fond opaque,
  suit le curseur (calque Foreground, non rogné par le scroll).
- Placeholder par défaut restauré ; `_blueprint.png` conservé.
- egui 0.31.1 depuis crates.io (retrait du patch local).

## 0.5.0

- Alignement de la version à 0.5.0 et première publication du dépôt sur
  GitHub.

## 0.3.0 — Refactor complet (architecture en couches)

Réécriture du cœur de l'application : le code monolithique
(`parser.rs`, `model.rs`, `fsops.rs`) est remplacé par quatre couches
indépendantes, avec la même interface utilisateur.

### Ajouté

- Couche `cfg` : lexage + document **sans perte** (roundtrip byte-exact
  vérifié ; BOM, CRLF et sauts de ligne préservés à l'export).
- Couche `config` : tous les paramètres du jeu sortent du code et vivent dans
  un TOML (`assets/config/default.toml` embarqué, surcharge par
  `<racine>/zoneedit.toml` — fusion des tables, remplacement des tableaux).
  Fichiers, clés cfg, champs (bornes, affichage %), paires min/max,
  catégories, règles de classification, filtres de type, actions en lot,
  rangs : plus aucune valeur du jeu en dur dans le code.
- Couche `domain` : modèles typés (`FieldKind`, `Entry`, `Block`,
  `Workspace`), extraction et classification pilotées par la config,
  **chemin d'édition unique** (`Workspace::apply` : normalisation →
  validation → undo/redo), validation des bornes et des paires.
- Couche `io` : chargement parallèle (un thread par fichier), export
  sécurisé (empreinte des sources, détection d'obsolescence, validation
  complète, écriture atomique `.tmp` + rename, nettoyage des anciens),
  presets v3 avec **contrôle de la valeur d'origine** (v1/v2 restants lisibles).
- Benchmarks `criterion` (`benches/core.rs`) : chargement ~18 ms, rendu d'un
  fichier ~75 µs, validation ~70 µs, export de 500 champs ~1,8 ms sur un jeu
  de données réaliste (~7 700 entrées).
- `check.sh` (fmt + clippy `-D warnings` + tests), `rustfmt.toml`, lints
  stricts dans `Cargo.toml`, doc complète des couches cfg / config / domain /
  io (`#![warn(missing_docs)]`).
- `README.md` (ce fichier).

### Modifié

- UI (egui) : portée telle quelle sur les nouveaux modèles ; suppression du
  fichier `cell.rs` (rendu des cellules fondu dans `rows.rs`).
- Les champs de groupe (calibre / set) et les rangs composés sont gérés par
  le domaine : correction du bug des rangs composés (l'ancien code réduisait
  le masque de rangs à « Master » ; le nouveau conserve tous les rangs).
- 57 tests (unitaires + intégration UI) ; le test de la touche Entrée
  (ignoré depuis le début) est corrigé et passe.

### Supprimé

- `src/parser.rs`, `src/model.rs`, `src/fsops.rs` (remplacés par les couches
  `cfg`, `domain`, `io`).
