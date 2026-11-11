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
