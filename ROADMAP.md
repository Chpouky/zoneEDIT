# zoneEDIT — Roadmap

Forward plan for the anomaly work. The **v1** basics (behaviour editing + parent/child
hierarchy) are shipped; everything under **Next up** is the **v2.x** line on top of them.

> Full data reference (field inventory, vanilla values, effect anatomy, reference chains
> per anomaly type): `.agents/skills/stalker-anomalies/references/anomaly-map.md`.

---

## Shipped

- **v0.8.0** — STATS « Anomalies »: `AnomalyPrototypes.cfg` loaded (one block = one
  entry), 11 behaviour fields editable, parent → child hierarchy (shared tint + "↳"
  prefix, always expanded), `block_entry_roles` config mechanism.

## Export fix (before v2 work)

- [x] **F — `{bpatch}` only on the root of each modified block**
      The merge is recursive from the root, so the keyword is required once
      per modified block (its top-level node) — nested scopes stay plain
      `struct.begin`. The previous exporter stamped `{bpatch}` on every
      `struct.begin` (redundant). Done: `src/io/export.rs` renderer + tests,
      skill doc updated.

## Data coverage (before v2 work)

Gaps found by auditing the loaded files against the vanilla GameData (fields and
files that are user-relevant but not yet mappable).

- [x] **G — Weapon durability range** (`MinDurability` / `MaxDurability`)
      Done: two `FieldKind` variants + `[[fields]]` (float 0..1) + `[[pairs]]`
      rule (min ≤ max); columns added to the `AmmoWeapon` and `NpcLoadout`
      categories.
      Item-scope fields on ~2 000 weapon rows across `ItemGeneratorPrototypes.cfg`,
      `DynamicItemGenerator.cfg` and `QuestItemGeneratorPrototypes.cfg` (the
      durability range of the spawned weapon, next to `AmmoMin/MaxCount`). Two new
      `[[fields]]` + a `[[pairs]]` rule (min ≤ max); expose the columns on the
      weapon categories (`AmmoWeapon`, `NpcLoadout`).
- [x] **H — World artifact spawners** (`ArtifactSpawnerPrototypes.cfg`)
      Done: new loaded file (93 spawners, one entry per block × rank, 364
      entries in vanilla) + a new **ARTIFACTS** sidebar section with an
      `ArtifactSpawner` category (10 config-only columns: `Count`, `Radius`,
      `MinCooldown`/`MaxCooldown`, `SpawnChanceBase`/`Bonus`, 4 `RarityChance`)
      and 9 new `FieldKind`s. Two new extraction modes, both config-driven via
      `[parser]`: `rank_scope_roles` (a scope named after a configured rank
      opens an entry for the whole block, scoped to that rank) and *transparent*
      structureless sub-scopes (`RarityChance`'s keys attach to the rank entry).
      The `Count`/`Common`/… keys are mapped with key aliases. The patch re-emits
      the keys exactly as written in the base file (see the export fix below).
      Rows show the icon of the spawner's first listed artifact: the
      `[parser] icon_list_scope` (`ListOfArtifacts`) scope is read during
      extraction and its first non-empty item becomes a display-only
      `Entry::icon_sid` hint (shared by the block's rank entries, never
      exported; the UI falls back to the placeholder when the artifact has
      no icon file or the list is empty).
      Tests: extractor unit test, fixture export-patch test (nested
      `Newbie`/`RarityChance` nodes, original keys), real-data byte-exact
      round-trip + icon-hint assertions, render smoke test.
- [ ] **I — Block-level generator fields** (`DynamicItemGenerator.cfg`)
      `RefreshTime` (e.g. `1h`) and `ReputationThreshold` (e.g. `-500`) live on the
      generator scope, not on items. Needs a "block-level field" notion in the
      domain (current model is entry/group scope only) and a non-numeric field type
      for `RefreshTime`.
- [x] **J — Misspelled keys in the vanilla data** (`weight`, `AmmoMaxcount`)
      Done: new `[parser.aliases]` table in the config (alias → canonical key,
      checked at compile time); `field_kind_for_key` resolves aliases. The
      exported patch keeps the original key (the `Field` now carries the key as
      written in the file; `render_patch` re-emits it — before this fix the
      canonical key was emitted, which the merge would have added as a NEW key
      instead of replacing `weight`/`Count`/`Common`).

## Weapon stats (before v2 work)

Research already done and documented — full file map, the weapons by class,
inheritance scheme and the inventoried editable fields (4 groups: GeneralSetup /
Settings / Attributes / Inventory): `.agents/skills/stalker-weapons/`
(SKILL.md + `references/weapons.md`).

- [x] **W — Weapon stats editing** (`GameData/WeaponData/*.cfg`)
      Load the 3 weapon files (vanilla audit, block counts verified on disk):
      - **GeneralSetup** (`WeaponData/WeaponGeneralSetupPrototypes.cfg`) —
        92 blocks: `TemplateWeapon` + 13 class templates + 78 weapons.
        **3-level** `refkey` inheritance (TemplateWeapon → Template<Class> →
        weapon), unlike anomalies (2-level).
      - **Player settings** (`WeaponData/CharacterWeaponSettingsPrototypes/
        PlayerWeaponSettingsPrototypes.cfg`) — 66 blocks (39 roots + 27 children);
        damage, ranges, bleeding + 6 `*UI` display stats.
      - **NPC settings** (`…/NPCWeaponSettingsPrototypes.cfg`) — 75 blocks
        (39 roots + 36 children); the NPC-side damage/spread diffs.

      **Extraction** — config-only, zero `extract.rs` changes: the 3 roles join
      `[parser] block_entry_roles` (same mechanism as anomalies). One block =
      one entry; keys under transparent structureless scopes (`RecoilParams/…`,
      `DispersionParams/…`) attach to the block entry automatically, and only keys
      with a configured `FieldKind` are extracted — everything else (VFX/audio/
      socket/enum/path/boolean) is ignored by design. Files use CRLF + `//`
      comments: already handled by the cfg lexer.

      **Multi-category membership (new domain capability, the core of W).**
      A GeneralSetup block defines fields of *every* stat group at once, so the
      one-category-per-entry model cannot split it into per-stat views. Decision
      (user-approved): an entry may belong to **several** categories, opt-in per
      file via a new `[[files]] multi_category = true` flag. `classify` returns
      *all* matching rules for such files (first-match-wins elsewhere, unchanged);
      `Workspace.categories` becomes `Vec<Vec<CategoryId>>`; the 3 UI row filters
      (`==`) become `contains`. Undo/redo, export, editing, search untouched
      (keyed on entries/fields). Generic — reusable by any future wide-block file.

      **Sidebar** (config-only, `section = "STATS"`, `group = "Weapons"`): 10
      sub-entries, each a stat view of the same blocks (4–18 columns each instead
      of one 52-column table). Templates stay visible (global defaults worth
      editing):
      - General — fire & cadence (6) / aiming & lean (8) / recoil (10) /
        dispersion (4) / jams (4) / reload & ammo (14) / heating & last clip (6)
        → 7 views over the 92 GeneralSetup blocks
      - Player — damage & ballistics (18) / display stats (6, rule carries
        `has_fields = ["AccuracyUI"]` so only blocks with `*UI` fields appear)
      - NPC — combat (12)
      Total: 77 new `FieldKind`s (shared across views, e.g. `BaseDamage` in both
      Player and NPC views), ~52 GeneralSetup + 18 Player core + 6 `*UI` + 1 NPC.

      **Other sub-tasks:**
      - Parser: `%` suffix for values like `ChanceBleedingPerShot = 10%`
        (`parse_value` + validation, like the existing `f` suffix; raw display).
      - Icons: `icon_for` fallback chain — strip `_Player`/`_NPC` suffix, then a
        `Guard` prefix, then the existing placeholder (covers
        `GunPM_HG_Player` → `GunPM_HG.png`, `GuardGunPM_HG_NPC` →
        `GuardGunPM_HG.png`; 56 GeneralSetup blocks, mostly templates, fall back
        to `_placeholder.png`).
      - Export: standard `{bpatch}` patch, one block per entry (item F's rules).
      - Fixtures: 3 small weapon cfg files in `tests/fixtures/GameData/`
        (load errors on a missing configured file), file-count assertions 6 → 9;
        unit tests for multi-category classify/membership and `%` parsing.

      **Out of v1 (→ E and later):** per-jam-level `WeaponJamParams.[N]` and
      per-magazine `WeaponReloadTimePerAttachment.[N]` (list-scope, item E), the
      per-state modifier subtrees (`RecoilParams/ShootingStateParams/…` — 30+ keys
      duplicated at block level in other blocks), VFX/audio/sockets/enums, the
      A/B inherited-value display & override keys (apply later, verbatim).

      **Implementation status (done):** all sub-tasks implemented and `./check.sh`
      green. Final shape, with the audit-driven deviations from the plan above:
      - **9 sidebar rows** (dispersion merged into the recoil view):
        fire & cadence (6) / aiming & lean (8) / recoil & dispersion (9) /
        jams (4) / reload & ammo (7) / heating & last clip (6) over the 92
        GeneralSetup blocks; player damage (18) / display (6) over 66 blocks;
        NPC combat (12) over 75 blocks.
      - **64 new `FieldKind`s** (96 total). Excluded after the value audit:
        `PostShotDelay` / `NumberOfShotsBeforeDelay` (do not exist in STALKER 2
        data), `DispersionRadiusMultiplierByDistanceCurve` (CurveFloat asset path
        in the NPC file), `BaseBleeding` unbounded (vanilla values 0..45),
        `UnloadTime` + `ProjectileSpawnOffset` added (flat numeric d1 keys).
      - `parse_value` strips a trailing `%` (vanilla
        `ChanceBleedingPerShot = 10%`); 4 new min/max pairs (jam chance /
        jam durability / dead-NPC ammo / effective fire distance).
      - Icon chain `icon_candidates`: exact SID → strip `_Player`/`_NPC` →
        strip `Guard` prefix (both covered by tests).
      - Fixtures: 3 weapon files (CRLF like the real ones, 3- and 4-space
        indents, list scopes, `%` values); byte-exact roundtrip + block-count
        (92/66/75) + vanilla-value tests in `tests/parser_test.rs`; headless
        render test of the weapon rows in `src/ui/tests/render.rs`.

## Traders page (before v2 work)

Audit of `GameData/ItemGeneratorPrototypes/DynamicItemGenerator.cfg` (already
loaded, role `dynamic_npc`): 64 `*TradeItemGenerator*` blocks = the in-game
shops, 534 rows (368 items — `Chance` on all, `MinCount`/`MaxCount` on 346,
`Min/MaxDurability` on 146 — no `Weight` — + 166 delegation references). Today
they leak into the item-spawn categories (armor, grenades, detectors, food,
artifacts, attachments, NPC loadouts — 40 of the 64 visible in 7 categories;
technicians/medics invisible). The 9 `GeneralNPC_TradeItemGenerator_*` faction
shops (currently under NPC Loadouts) move to the new page too.

- [x] **T — Traders page** (`*TradeItemGenerator*` blocks, grouped by location)
      A dedicated **TRADERS** sidebar section: one category per location
      (Zalesie, Terricon, Sultansk, Malakhit, Rostok, Yanov, Asylum, Ikar, Newbie
      Village, Cement Plant, Chemical Plant, Pripyat, Shevchenko, Varta, Backwater,
      Red Forest, Eger, Suska, Selma, Power Plug — 20) plus a **Universal /
      Other** category (universal shops + the 10 NPC faction traders) = 21
      categories. 5 columns each: `Chance`, `MinCount`, `MaxCount`,
      `MinDurability`, `MaxDurability` (existing `FieldKind`s — no new field
      type).
      - **Classification**: new `exclude_block` matcher on `[[rules]]` (a rule
        matches only when the block does NOT match the pattern) — the 10
        item-spawn rules that currently capture trade blocks get
        `exclude_block = { contains = ["TradeItemGenerator"] }`; the 21 new
        TRADERS rules match the blocks by name prefix (block names are not
        uniform: `TraderSuska`, `Bartender_ChemicalPlant`, …). Config-only
        otherwise.
      - **Rendering (Item Spawns model)**: each shop is a regular foldable
        generator row (collapsed by default, like Food & Drink); its items
        are plain rows inside the dropdown (no "↳" — the items are not
        children of the shop, unlike the anomaly/weapon hierarchy), the
        delegation rows keep the "↳ target" label. No trade-specific code in
        the renderer: block detection and the page are config-only.
      - **Trade filter**: the existing `st.view.trade` topbar filter stays —
        it is a cross-section search ("trade" in SID/block/context),
        independent of the new section.
      - Tests: `fixture_traders` (classification + `exclude_block` +
        delegation), headless render test `traders_rows_render` (dropdown
        row + plain item rows after expand + delegation row + 5 headers),
        real-gamedata `real_traders_page`.

- [x] **T2 — Every Trader* block on the TRADERS page**
      The 64 shop blocks were not the whole story: **97 blocks** in the
      loaded files carry a trader name — the 64 shops plus the **shared
      sub-pools** the shops delegate to (`Trader_T1…T4_Guns/Ammo`,
      `Trader_NatoAmmo/SovietAmmo`, `Trader_Attachments_T2…T4`,
      `Trader_Cosnsumables`, `TraderSecondary/PrimaryWeapon`,
      `DynamicTrader`, `AllTrader`, `MainTraderItemGeneratorV1`,
      `RC_TraderNPC`) and the **wandering-NPC-trader generators**
      (`SvoraTrader`, `VoentorgTrader`, `SkadovskTrader`, `TraderEugene`,
      `TraderJunker` → Universal / Other; `ZalesieTrader` → Zalesie,
      `TerriconTraderGuron` → Terricon, `supack_trader` → Selma,
      `HimzavodTrader` → Chemical Plant, `TraderSuska` → Suska).
      Naming rule (zero collateral in the loaded files): a block whose name
      contains **"Trade" or "trader"** belongs to the TRADERS page. The 10
      item-spawn rules exclude them (`contains = ["Trade", "trader"]`),
      the TRADERS location rules grew their prefixes accordingly —
      config-only.
      - The TRADERS page is **uncolored**: the delegation tint (used to
        group a stock seen in two places, e.g. Medical Crates) is skipped
        for rows of the TRADERS section — a shop → shared-pool delegation
        is a plain data reference. Item Spawns tints are untouched.
      - Tests: `real_traders_page` (97 blocks, 847 rows, 0 leakage in both
        directions), `fixture_traders` gained a shared-pool block
        (`Trader_Food_Fixture_ItemGenerator` — excluded from Food,
        classified Universal Other).

## Next up (v2) — ordered by dependency

- [ ] **A — Show inherited values in sub-rows**
      A child that doesn't define a field shows the value it **inherits**, greyed and
      non-editable. *Foundation for B.*
- [ ] **B — Per-variant override keys** *(needs A)*
      Add a key to a child block so it differs from its parent **without touching the
      parent** (e.g. `Radius = 150` in the Small block). Trigger: click the inherited
      (greyed) cell → *Override…*.
- [ ] **C — Effect values** (`EffectPrototypes.cfg`)
      Edit the **damage / corrosion** numbers an anomaly references (incl. composite
      effects). Independent of A/B — a new file + an effect model.
- [ ] **D — Swap the `ItemPrototypeSID` of an item entry**
      Today only the values of an item entry are editable (`Chance`, `Weight`,
      `MinCount`, `MaxCount`, …) — the `ItemPrototypeSID` itself is fixed. This lets
      the user change the SID inside an item struct (`PossibleItems.[N]`) so a
      different object spawns in that slot, by **picking from a list of existing item
      SIDs, grouped by category** (food, armor, ammo, …).
- [ ] **E — Add / remove item nodes in item lists** *(adding uses D's catalog)*
      Item lists (`PossibleItems` with its `[0]`, `[1]`, … entries — present in
      stashes, item generators and the other spawn files, not only stashes) are
      currently fixed in size: only the values of existing entries are editable.
      This lets the user **remove an entry** (exported as `[N] : removenode` inside
      the item-list scope) or **add a new entry** (a new numbered node with its
      fields, SID picked via the D catalog).

**A → B** are a pair: A makes the inherited value visible, B lets you turn a greyed
cell into an override. **C** is standalone. **D → E** form the spawn-list work:
D swaps the item of an existing slot, E changes the size of the list itself.

## Windows build — sluggish window dragging

Reported on a Windows machine: the app works, but moving the window around the
desktop feels laggy. Root-cause investigation (no fix yet) — ranked by
likelihood:

**The amplifier.** Stock `eframe::run_native` with no repaint throttling
(`src/main.rs`): eframe 0.31 repaints on demand, but egui requests a repaint on
*any* input event (pointer motion included), and Windows emits redraw events on
every window-position change. While dragging (or just hovering with a
high-polling mouse / 144 Hz+ display), the whole `update()` runs on every
event.

Per-frame costs in `update()`, all static between edits but re-run on every
repaint:

- **Disk I/O every frame** (most Windows-specific): `show_editor` calls
  `io::load::stale_files(ws)` unconditionally (`src/ui/mod.rs`) →
  `std::fs::metadata` on all 6 game files per frame. On Windows each stat goes
  through `GetFileInformationByHandle` and is hooked by Windows Defender
  real-time protection — easily a few ms per frame, more if the game folder is
  on a secondary drive.
- **Full-workspace recompute every frame** in `compute_derived`
  (`src/ui/mod.rs`): `visible_indices` scan, `anomaly_hierarchy` (rebuilds
  HashMaps, clones every block name), `n_err` over ALL fields of the workspace
  (not just visible), `category_counts` full scan.
- **`anomaly_hierarchy` computed twice per frame** — once in
  `compute_derived`, once again in `render_table` (`src/ui/table.rs`).
- **No row virtualization**: the table is a plain `egui::ScrollArea`; every
  row of the selected category is laid out each frame (thousands for
  `ItemGeneratorPrototypes.cfg`), not just the ~20 on screen.

Minor / one-time (probably not the culprit): 324 icon PNGs (~21 MB) are decoded
lazily and cached by egui (one-time cost after loading a project); glow/OpenGL
renderer and the release profile are fine.

**How to confirm on the Windows machine.** The status bar shows "frame X ms":
high values while merely moving the mouse → per-frame work. A Defender
exclusion for the game folder + SSD should isolate the `stale_files` cost.

Candidate fixes (each independent, none done yet):

- [ ] Stale-check at most every N seconds (and on window focus), not every frame
- [ ] Compute `derived` / `anomaly_hierarchy` only when the workspace revision
      or the view filters change (cache in `EditorState`)
- [ ] Deduplicate the second `anomaly_hierarchy` call in `render_table`
- [ ] Cap repaints (`request_repaint_after`) so input events can't force
      more than ~60 fps of updates

## Backlog / later

- [ ] `AnomalyOverlapPrototypes.cfg` — the anomaly coexistence matrix
- [ ] Enum pickers for identity fields (`Type`, `AnomalyElementType`) — free text today
- [ ] More anomaly behaviour fields beyond the current 11 — full data audit done
      (every top-level key of all 26 blocks). **18 gameplay candidates**, grouped
      by family — pick a small subset per release rather than all at once
      (29 total columns would be too wide):
      - chase / projectiles (FireBall, LightningBall ×3): `HuntDistance`,
        `EMIRadius` / `EMIDuration`, `CloseArtifactRadius`,
        `AnomalyInterpolationSpeed`, `AnomalyStuckTime`
      - Razor: `DamageEffectSpawnChance` (0..1, percent display)
      - LavaLamp: `VoidRadiusMin` / `VoidRadiusMax` (min ≤ max pair),
        `DestructibleDamage`, `PushingImpulse`
      - Expulsion: `PullForce`, `ZeroGravityRadius`
      - Carousel: `LiftingHeight`, `LiftUpTime`, `TimeToCenter`
      - Pillow / Springboard: `DefaultAnomalyRadius` · Diamond:
        `EquipmentDamage`
      - excluded: VFX / particle / SFX / animation internals (~60 keys), deep
        artifact-eating mechanics (`ArtifactEating*`, `MaxArtifactsToUpSpeed`),
        push-impulse physics, `ToxicCloud*` timers (4 keys, 1 family), booleans
        (→ Boolean field type item), `MinHP` / `MaxHP` / `MinSpeed` /
        `MaxSpeed` (nested `ParticlePrototype` / `BubblePrototype` sub-scopes —
        outside the flat-key model)
- [ ] Boolean field type — `bAllowSameCategoryGeneration`, `bRequireWeapon`,
      `bRequireAmmo`, `bUnloadedWeapon`, `bHasSpaceRestrictor`, … (the field type
      system is `int` / `float` only today)
- [ ] Placeholders / preplaced items (`SpawnActorPrototypes.cfg` + per-map folder
      tree, ~130k structs): `SpawnOnStart`, `SpawnedPrototypeSID`,
      `DamageMultiplier`, `MoveSpeedMultiplier`, `HP` — a distinct model (no item
      scope; deletion via `removenode`), a big standalone feature
- [ ] Quest subfolders — `QuestNPCGeneratorPrototypes/`,
      `QuestStashBodyItemGeneratorPrototypes/`, `QuestRewardsPrototype(s)`

---

# Appendix — design & data notes

Details behind A/B/C/D/E (user stories, constraints, sub-tasks, verified inheritance
facts). Context for the plan, not the plan itself.

## A — Show inherited values in sub-rows (greyed, non-editable)

**User story.** In the anomaly sub-row hierarchy, a child that does not define a field
currently shows an empty cell. This shows the value it **inherits** from its parent,
rendered **greyed and non-editable** — so the table documents the inheritance (e.g.
`LightningBallSmallAnomaly` visibly inherits `Radius = 100`, `Cooldown = 4` from
`LightningBallMediumAnomaly`). It pairs directly with B: a greyed inherited cell is what
the user clicks to create an override.

**Constraint.** Requires resolving the `refkey` chain to fetch the parent's effective
value for display; v1 keeps sub-rows to "the fields the block itself defines".

**Sub-tasks.**
- Domain: resolve a field's effective value by walking `refkey` up to the block that
  defines it (vanilla inheritance only, not mod-patched values).
- UI: a distinct cell rendering mode — *inherited* (greyed, non-editable) vs *present*
  (editable) vs *absent*.

> Inheritance facts (`refkey`, depth, the 4 families) live in **AGENTS.md →
> Anomalies** (durable reference) + the anomaly data map.

## B — Per-variant override keys (add a key to a child block)

**User story.** A variant inherits its values from its parent and, for any field it
does not declare itself, it cannot be made different. E.g. `LightningBallSmallAnomaly`
inherits `Radius = 100` from `LightningBallMediumAnomaly` and has no `Radius` key of
its own, so today it cannot be given a different radius. This lets the user **add an
override key to the variant block** so it differs from its parent *without touching the
parent* — writing `Radius = 150` into the Small block. In the exported `{bpatch}` that
new line sits at the block's real depth, which the game resolves as an override.

**Constraint.** The current model is "edit existing values only": it has no notion of
*creating* a key, and the export re-emits only dirty *existing* fields.

**Sub-tasks.**
- Domain: add a key/value to a block (not just mutate an existing field), keeping
  counters, revision, undo/redo and validation consistent.
- UI: the natural trigger is "click an inherited (greyed) cell → Override…" (see A), or
  an explicit "add override" action on the row.
- Export: emit the new key inside the block's `{bpatch}` at the correct depth.
- Guard: only allow overriding a key that exists somewhere in the inheritance chain
  (never invent an unknown key).

## C — Expose damage / effect values (`EffectPrototypes.cfg`)

**User story.** Anomaly *behaviour* (radius, cooldown, …) is editable in
`AnomalyPrototypes.cfg`, but the actual **damage / corrosion / status numbers** live in
`EffectPrototypes.cfg` (2425 top-level blocks). An anomaly block only stores SID
*references* to its effects (e.g. the `InteractionEffectPrototypeSIDs` list). This lets
the user edit those effect values (damage `ValueMin`/`ValueMax`, `DamageType`,
`ArmorPiercing`, `Duration`/`Charges`, …) from zoneEDIT.

**Constraint.** Needs a new file loaded, SID-reference resolution, and handling of
**Composite** effects (an effect such as `ElectroCorrosion` nests sub-effects
`_Weapon` / `_SecWeapon` / `_Pistol` / `_Body` / `_Head`). A distinct feature from the
anomaly behaviour editing.

**Sub-tasks.**
- Load `EffectPrototypes.cfg` (a new file in the config).
- Resolve anomaly → effect through the `*EffectPrototypeSIDs` reference fields
  (7 such fields — see the data map §3.1).
- Handle Composite nesting (nested sub-effect blocks).
- Export: emit patches for the effect blocks, following that file's patch convention
  (cfg_patch style — see the data map).

## D — Swap the `ItemPrototypeSID` of an item entry

**User story.** A spawn slot is defined by its `ItemPrototypeSID` (e.g. `Vodka` in
`DestructibleStashProlog_Drink`). The user wants to make that slot spawn a
different object (e.g. `WaterBottle`) without touching anything else. Today the
app only edits the item's *values* (`Chance`, `Weight`, `MinCount`, `MaxCount`, …);
the SID itself is fixed. The feature lets the user replace it by **choosing in a
list of existing SIDs, classified by category** (food, armor, ammo, …) — no free
text, so an unknown SID can never be exported.

**Constraint.** The candidate list must cover the game's full item prototype space
(grouped by item category), not the spawn files themselves (which only ever list a
subset). **There is no clean list of all items today** — the SIDs are scattered
across the `.cfg` files, so the catalog has to be **created**: built once from the
vanilla item prototype data (the same family of files the icons/SID lookups already
use), then bundled with the app as the single source for the picker.

**Sub-tasks.**
- Data: load/build the item catalog (SID → display name, category, icon where
  available) and expose it to the UI.
- Domain: make the `ItemPrototypeSID` field editable via the Workspace (undo/redo,
  revision, counters stay consistent), constrained to SIDs present in the catalog.
- UI: a picker on the `ItemPrototypeSID` cell — categories as top-level groups
  (food, armor, ammo, …) + search; the current SID is shown as the selected value.
- Export: unchanged mechanism — the new SID is a dirty field emitted at its real
  depth, like any other value change.

## E — Add / remove item nodes in item lists

**User story.** An item list (a `PossibleItems` scope with `[0]`, `[1]`, … item
entries — found in stashes, item generators, quest generators and the other spawn
files, not only stashes) is currently fixed in size: only the values of the existing
entries can be changed. The user wants to **remove an entry** (e.g. drop `CannedFood`
from a drink stash) or **add a new entry** (e.g. put a medkit into a stash that
never had one).

**Constraint.** The current model is "edit existing values only": it has no notion of
creating or deleting a node (same gap as B, which creates keys; E creates/deletes
whole nodes). Deletion is exported with the `removenode` keyword, which is only
valid inside the `{bpatch}` merge chain (see the patching reference, section 2). This
also means the export is no longer "only dirty fields": it can now carry structural
changes.

**Sub-tasks.**
- Domain: delete an item node, and add a new item node to a `PossibleItems` scope
  (next free index `[max+1]`), keeping counters, revision, undo/redo and validation
  consistent.
- UI: row actions — *remove* on an item entry; *add* on the list (new entry with its
  values, SID picked via the D catalog picker).
- Export: removed entries are emitted as `[N] : removenode` inside the rebuilt item-
  list scope (root keeps `{bpatch}`, nested scopes plain); added entries are emitted
  as full numbered nodes at their real depth. Coexistence of add + remove + value
  edits in a single patch must work (one scope, several operations).
- Guard: never export a `removenode` for an index that doesn't exist in the base;
  confirm the game's merge keeps list indices stable when appending (a new entry is
  `[max+1]`, never a renumbering).
