# Changelog

## 0.11.0

### Features

- **Traders page**: the 64 trader generator blocks of
  `DynamicItemGenerator.cfg` (the in-game shops) get a dedicated **TRADERS**
  sidebar section, grouped by location — 20 location categories (Zalesie,
  Terricon, Sultansk, Malakhit, Rostok, Yanov, Asylum, Ikar, Newbie Village,
  Cement Plant, Chemical Plant, Pripyat, Shevchenko, Varta, Backwater, Red
  Forest, Eger, Suska, Selma, Power Plug) plus **Universal / Other** (universal
  shops + the NPC faction traders). Each shop is a regular foldable generator
  row (same layout as Item Spawns — expand it to see the stock): the items
  are plain rows with the columns **Chance**, **Min Count**, **Max Count**,
  **Min Dur**, **Max Dur** (delegation references to other generators are
  shown as "↳ target" rows). The trade blocks no longer leak into the
  item-spawn categories (armor, grenades, detectors, food, …) — classification
  is config-only: a new `exclude_block` rule matcher on `[[rules]]` + the 21
  TRADERS categories/rules.
- **All Trader* blocks moved to the TRADERS page**: not only the 64 shops —
  every trader-named block of the loaded files (97 total: the shared
  sub-pools the shops delegate to, e.g. `Trader_T2_Guns`, and the
  wandering-NPC-trader generators) now lives on the TRADERS page, each in
  its location category or **Universal / Other** when no location applies.
  Rule of thumb: a block whose name contains "Trade" or "trader" is a
  trader block — none of them appears in the item-spawn categories anymore
  (weapons loaded ammo, food & drink, grenades, armor & NVG, attachments,
  detectors, …). The TRADERS page is also uncolored: the delegation row
  tint is skipped there (it only serves to group a stock seen in two
  places, like the Medical Crates on Item Spawns).

### UI

- Weapon hierarchy: child rows now show an icon when a real one exists —
  the gun's own, or inherited from the closest ancestor with one (a gun
  variant reuses its gun's icon, whatever the depth). A child with no icon
  anywhere in the chain (anomaly variants, family templates) stays
  icon-less: the "↳" arrow and the family tint already group it with its
  parent.
- The three batch buttons of the batch bar ("Set Chance…", "Set Min/Max…",
  "Set Min/Max Spawn %…") are no longer shipped: the bottom bar only shows
  the selection counter, "Reset Selected", "Reset All" and the PRESET
  dropdown. The feature itself is kept — batch buttons are config-only
  (`[[batch]]` entries), so re-adding one restores the button; the headless
  tests inject the specs via a config override in the meantime.

## 0.10.0

### Features

- **Weapon stats editing**: the three `GameData/WeaponData` files are loaded —
  `WeaponGeneralSetupPrototypes.cfg` (92 blocks),
  `PlayerWeaponSettingsPrototypes.cfg` (66) and
  `NPCWeaponSettingsPrototypes.cfg` (75). One block = one weapon (same
  block-entry mechanism as anomalies). New **Weapons** group in the STATS
  sidebar with 9 stat views: **fire & cadence**, **aiming & lean**, **recoil
  & dispersion**, **jams & durability**, **reload & ammo**, **heating & last
  clip** (GeneralSetup), **damage & ballistics** + **display stats** (player)
  and **combat stats** (NPC) — 4–18 columns each instead of one 50+ column
  table. A weapon row appears in every view of its file (new per-file
  `multi_category` config flag; an entry can now belong to several
  categories).
- **Percent values**: cfg values with a `%` suffix (`ChanceBleedingPerShot =
  10%`) are parsed, validated and editable (like the `f` suffix).
- **Weapon icons**: one icon per base weapon is shared by its variants —
  `GunPM_HG_Player` / `GuardGunPM_HG_NPC` resolve to `GunPM_HG.png` /
  `GuardGunPM_HG.png`; blocks without any icon keep the placeholder.
- **Dynamic column widths**: value columns are sized on their header labels
  (no more ellipsis truncation) and the table scrolls horizontally when a
  category's columns (plus the name zone) are wider than the window — the
  sticky header, the row names and the editable cells all scroll together.
  Wide categories such as the 18-column **Player Damage** view are no longer
  clipped on small windows.

### UI

- **Sidebar row names**: shorter weapon names — **Firing**, **Aiming**,
  **Recoil**, **Jamming**, **Reloading**, **Heating** — and Title Case for
  every multi-word row (e.g. *Player Damage*, *NPC Combat*, *Medical
  Crates*, *Ammo Loot*). Config-only (`short` labels in `default.toml`).
- **Pinned horizontal scrollbar**: the table's horizontal scrollbar now
  stays at the bottom of the row window instead of floating under the last
  row when a category has few rows (egui `auto_shrink(false)`).
- **Header/rows scroll lockstep**: the sticky column labels no longer lead
  the rows by one frame while scrolling — they use the same pre-update
  scroll offset the rows are drawn with.
- **Hierarchy indent**: nested rows (anomaly variants, weapon family
  templates → guns) are indented to the right — one "↳" marker for every
  child and *depth* × 14 px of offset, so the nesting is visible at a
  glance. The checkbox and the value columns stay aligned; only the name
  zone moves. New UI rule: any future nesting in the table reuses this
  model (`child_depth` + `HIER_INDENT`), never a new arrow/indent.

### Bug Fixes

- **One color per weapon family**: the tint palette grew from 6 to 13
  slots (`c::GROUP_TINTS`), and a "color unit" is now a block that has
  children **and** sits at depth 0 or 1 of the `refkey` tree — the 10
  weapon family templates + `TemplateWeapon`, each with its own color.
  Guns and their variants (the real file is 4 levels deep, 29 guns are
  parents of variants) **inherit the family tint** instead of cycling
  through the 6 colors — pistols are one color, SMGs another. Also fixed:
  the tint was only written for the unit blocks, so guns and anomaly
  variants were actually painted **untinted**; it is now written for every
  visible block (each row reads its own block tint).
- **Specific weapons now visible in the six GeneralSetup views** (Firing,
  Aiming, Recoil, Jamming, Reloading, Heating): the inheritance hierarchy is
  walked **recursively** at any depth — the weapon GeneralSetup is 3 levels
  deep (`TemplateWeapon → family template → gun`), and the level-3 rows
  (all ~79 guns) were previously skipped: only the template rows were
  painted. A child row at depth *d* carries one "↳" marker and is indented
  right by *d* levels (offset = depth, never a stack of arrows), and a
  child whose parent is filtered out (e.g. by the search) is still shown —
  a search for a gun
  name no longer returns an empty table.
- **Windows release build**: the icon paths generated for `include_bytes!`
  now use forward slashes — Windows backslashes were parsed as string
  escapes and broke the CI compile.

## 0.9.0

### Features

- **World artifact spawners**: new **ARTIFACTS** section in the sidebar and
  **Artifact spawners** category — the `ArtifactSpawnerPrototypes.cfg` file
  (93 spawners) is loaded: **one row per spawner × rank** (Newbie → Master,
  364 rows in vanilla) with 10 columns — **Count**, **Radius**,
  **Min/MaxCooldown**, **spawn chance** (base/bonus) and the 4 **rarity
  chances** (Common/Uncommon/Rare/Epic, `RarityChance` sub-scope in the cfg).
  Rarity values are 0..1 fractions, as in the game file. Each row shows the
  icon of the **first (non-empty) artifact** in the spawner's
  `ListOfArtifacts` list (the spawner SID has no icon of its own; spawners
  whose first artifact is missing or has no icon file — 8 in vanilla — keep
  the generic icon).
- **cfg key aliases**: the game's misspelled keys — `weight` (lowercase, 152
  lines in `ItemGeneratorPrototypes.cfg`) and `AmmoMaxcount` (4 lines in NPC
  loadouts) — are now read as **Weight** / **AmmoMaxCount** and are editable —
  new `[parser.aliases]` table in the config (alias → canonical key, checked
  at load time). The export keeps the original key, as written in the file.
- **Weapon durability**: new **MinDurability** / **MaxDurability** fields
  (0..1 fraction of max durability) editable on weapon rows — **Weapons
  Loaded Ammo** and **NPC loadouts** categories. The min ≤ max consistency is
  checked at validation (like MinCount/MaxCount).

### UI

- **Non-overlapping column headers**: the labels of the 9 new artifact
  spawner columns are shortened to fit the ~70 px column grid ("Min cool",
  "Base", "Bonus", "Common"…; the full name stays in the edit modal). On the
  stash pages, "Min Spawn %" / "Max Spawn %" become "**Min %**" / "**Max %**"
  (next to "Min Count" / "Max Count") — the old "Max Spawn %" already
  overflowed a few pixels onto its neighbor. Guard in the header painting: a
  label wider than the column pitch is truncated with "…" (`fit_label`,
  already used for row names) instead of being left overlapping.

### Fix

- **Export (aliased keys)**: patches re-emitted the field's **canonical key**
  (`Weight`, `ArtifactCount`…) instead of the **base file's key** (`weight`,
  `Count`, `Common`…). ZoneKit's merge would then have **added a new key**
  instead of replacing the value — the edit would have had no effect in the
  game. The field now carries the key as written in the file; the patch (and
  the full rendering, already correct) re-emits it verbatim. Dedicated
  export test.
- **Export**: the `{bpatch}` keyword is now emitted only on the **first
  node** (root) of each patch — the merge being recursive, nested scopes are
  plain `struct.begin`. The exporter used to put it on every `struct.begin`
  (redundant). Export tests updated; reference doc (skill
  `stalker-cfg-patching`) corrected accordingly.

## 0.8.0

### Data: STATS section "Anomalies"

- New **STATS** section and **Anomalies** category: the
  `AnomalyPrototypes.cfg` file is loaded as-is, **one block = one entry**.
- 11 editable behaviour fields per anomaly: **Radius**, **Cooldown**,
  **Duration**, **ActivationDelay**, **ActivationSpeedThreshold**,
  **DefaultHeightOffset**, **CollisionHeight**,
  **MovementLocationPriorityRadius**, **MovementSpeed**,
  **FakeArtifactSpawnAmountMin** / **FakeArtifactSpawnAmountMax**.
- Anomaly **hierarchy** (parent + `_Dynamic`/child variants): children share
  their parent's **tint** and carry a **"↳" prefix** glued to the name (same
  column, no icon), on the model of the delegation rows (Medical Crates).
- New config mechanism **`block_entry_roles`**: for these file roles, the
  level-1 block is itself the edited unit (no `ItemPrototypeSID`) — the
  fields are the flat keys under the block.

### UI

- **Row layer rewrite**: geometry per row type (`RowLayout`) + shared chrome
  (`row_frame`) + all constants in `metrics.rs`. Item, generator, rank, set,
  orphan and anomaly rows all go through the same mold — no more
  hand-coded row painting.
- Default window size increased (1400×720 → **1500×820**).

### Misc

- Legacy French comments translated to English.
- Tests: anomaly hierarchy rendering + parsing (`AnomalyPrototypes.cfg`
  fixture).

## 0.7.5

### UI

- Sidebar: the "ITEM TYPES" label and the search field are replaced by a
  **section dropdown**. Sections are config-driven (`section` field of
  `[[categories]]`, default `ITEM SPAWNS`): each section groups its
  categories, and the list adapts to the selection.
- The **search field** is moved to the **filter bar** at the top (right
  side), next to the file and reference filters; it works in combination
  with all filters.

## 0.7.4

### Export (full rewrite)

- The export generates **ZoneKit-compliant patches**: one `<Mod>.cfg` per
  modified file, placed in the prototype sub-folder that mirrors the base
  path (`GameData/X.cfg` → `GameData/X/<Mod>.cfg`) — no more full file
  mirroring.
- Each modified block is patched with the **`{bpatch}`** keyword: only the
  changed fields are written, **at their real depth** (the scope chain
  between the block and the field is reconstructed as nested `{bpatch}`
  nodes); the other fields of the base file are untouched.
- The node is targeted by its name as written in the base; reference
  attributes (`{refkey=…}`, `{refurl=…}`) are no longer re-emitted, and no
  more header comment in the exported file.
- No aggressive cleanup: zoneEDIT only writes (rewrites) its own
  `<Mod>.cfg` — other files in the mod are never touched.
- Tree fix: the content root is `<mod>/Content/GameLite` (base paths already
  contain `GameData/`) — no more doubled `GameData/GameData`; an existing
  structure (ZoneKit source, `GameData` at root, or the GameData folder
  itself) is detected rather than recreated.

### UI

- The "Mod name" field is removed: the **Export** button opens the folder
  browser and the mod name is the **name of the chosen folder**.
- The last chosen folder is remembered (`~/.config/zoneedit/state.toml`)
  and reloaded automatically at startup; the browser only opens if there
  isn't one yet.
- Status bar: the "struct.begin / struct.end balanced" indicator is removed.

## 0.7.3

### Misc

- Translation of the entire comment set (module headers, doc comments,
  inline comments, panic/test messages) from French to English —
  convention: the whole codebase is commented in English. No behavior
  change.

## 0.7.2

### Filters

- Filter bar renamed **"Filters"**, the single-selection filter is dropped in
  favor of **cumulative filters** (union): multiple ranks, "No Rank" and
  "Trade" can be freely combined.
- **"All" chip removed**: no active filter = show everything; one or more
  active filters = union of the matches.
- New **"Trade"** filter: only shows entries containing "trade" (SID, block
  or context, case-insensitive) — trader inventories.

### Batch bar

- "Reset" button renamed **"Reset Selected"**: only resets the selected rows
  (no effect without a selection).
- New **"Reset All"** button: resets every value in the workspace.

### Diff preview

- Popup redone: changes are **grouped by item** (one row per item, icon +
  name + changed values), grouped by file then by block, instead of one row
  per field.
- No more collapse arrow on the window.

### Misc

- Binoculars icons (Binoculars_01/02/03).
- Frame time shown in the status bar.
- Clippy cleanup (`-D warnings` green): test modules at the end of files,
  inline docs and formats.
- New `AGENTS.md`: architecture and conventions guide for coding agents.

## 0.7.1

### Icons

- **Weapon** icons from the new UI export (the `_body` variants,
  non-upgraded), players and guards; 20 weapons from the old export remapped
  (AK74 + Strelok variant, AKU, Silence SMG, M16, Obrez, PM + SkifGun, TOZ,
  Viper, G37 + V2…).
- Removal of the 80 redundant `Blueprint_*.png` (identical to the
  `_blueprint.png` fallback): lighter binary.
- `build.rs`: per-PNG-file `rerun-if-changed` (the folder watch ignored
  added icons).

### Misc

- Rank label always visible (even when a rank filter is active).

## 0.7.0

### Difficulty filter

- Ranks encoded in the **block name** are now taken into account (`_Rank`
  suffix and CamelCase, e.g. `Master`), in addition to the `Rank` /
  `PlayerRank` field which stays priority.
- When a rank is selected, entries **without a rank** are hidden.
- New **"No Rank"** filter: only shows entries without a rank.

### Classification

- Anomaly detectors (**Bear, Gilka, Veles, Echo, Oko**) move out of
  Loot/Artifacts/Accessories into a dedicated spawn source **"Detectors"**
  (Equipment).

### Cleanup

- **Type filter removed** for good (UI, state, config, logic). Old configs
  containing `[[type_filters]]` are explicitly rejected.

### Icons

- Items whose SID contains "blueprint" (case-insensitive) show the
  `_blueprint.png` icon instead of the placeholder.

### Search field

- **"×"** clear button on the right of the field: greyed and inactive when
  the field is empty, normal otherwise (same states as the batch buttons).
- Field text vertically centered by the layout (no more absolute
  positioning).
- The cross is **drawn by the Painter** (two diagonal segments at the exact
  center of the button): the "×" glyph of the font is not centered in its
  line box (measured ink +1.5 px below center), the geometric drawing
  guarantees exact centering, independent of the font.

### Tests

- 70/70 (46 lib + 24 integration); headless UI tests adapted (rank filter,
  "×" button, cross as two `LineSegment`s).

## 0.6.0

- Per-item icons (260 PNGs from the game exports, named by SID), embedded at
  compile time (`build.rs`) and lazily decoded via `egui_extras`.
- Vertical icon fitting (ratio capped at 2.0, ref. M16); item names aligned
  on a fixed column, whatever the icon.
- Popup on icon hover: image at original size on an opaque background,
  follows the cursor (Foreground layer, not clipped by the scroll).
- Default placeholder restored; `_blueprint.png` kept.
- egui 0.31.1 from crates.io (local patch dropped).

## 0.5.0

- Version aligned to 0.5.0 and first publication of the repository on
  GitHub.

## 0.3.0 — Full refactor (layered architecture)

Rewrite of the application core: the monolithic code (`parser.rs`,
`model.rs`, `fsops.rs`) is replaced by four independent layers, with the
same user interface.

### Added

- `cfg` layer: lexing + **lossless** document (byte-exact roundtrip
  verified; BOM, CRLF and line endings preserved on export).
- `config` layer: every game parameter leaves the code and lives in a TOML
  (embedded `assets/config/default.toml`, overridden by
  `<root>/zoneedit.toml` — table merge, array replacement). Files, cfg
  keys, fields (bounds, % display), min/max pairs, categories,
  classification rules, type filters, batch actions, ranks: no more game
  values hard-coded in the code.
- `domain` layer: typed models (`FieldKind`, `Entry`, `Block`,
  `Workspace`), config-driven extraction and classification, **single edit
  path** (`Workspace::apply`: normalization → validation → undo/redo),
  bounds and pairs validation.
- `io` layer: parallel loading (one thread per file), safe export (source
  fingerprints, stale detection, full validation, atomic `.tmp` + rename
  write, cleanup of outdated files), presets v3 with **original value
  checking** (v1/v2 still readable).
- `criterion` benchmarks (`benches/core.rs`): loading ~18 ms, one file
  rendering ~75 µs, validation ~70 µs, 500-field export ~1.8 ms on a
  realistic dataset (~7 700 entries).
- `check.sh` (fmt + clippy `-D warnings` + tests), `rustfmt.toml`, strict
  lints in `Cargo.toml`, full docs for the cfg / config / domain / io
  layers (`#![warn(missing_docs)]`).
- `README.md`.

### Changed

- UI (egui): ported as-is to the new models; removal of the `cell.rs` file
  (cell rendering merged into `rows.rs`).
- Group fields (caliber / set) and compound ranks are handled by the domain:
  fix of the compound-ranks bug (the old code reduced the rank mask to
  "Master"; the new one keeps all ranks).
- 57 tests (unit + UI integration); the Enter-key test (ignored since the
  beginning) is fixed and passes.

### Removed

- `src/parser.rs`, `src/model.rs`, `src/fsops.rs` (replaced by the `cfg`,
  `domain`, `io` layers).
