//! zoneEdit integration tests on the real `GameData/` data (parent
//! directory of `CARGO_MANIFEST_DIR`). Numeric comparisons go through
//! `domain::value::parse_value` (original values: "1.f", "0.", …).

use std::path::{Path, PathBuf};

use zoneedit::config::{CategoryId, Config};
use zoneedit::domain::value;
use zoneedit::domain::{EntryId, EntryKind, FieldKind, Workspace};
use zoneedit::io::{export, load, presets, stale_files};

fn load_root() -> Workspace {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("dossier parent de zoneEdit");
    load::load(root).expect("chargement de la racine des données")
}

/// Identifiant d'une entrée par (bloc, item) ; panique si absente.
fn find_id(ws: &Workspace, block: &str, item: &str) -> EntryId {
    ws.entry_ids()
        .find(|&id| {
            let e = ws.entry(id);
            ws.block(e.block).name == block && e.sid == item
        })
        .unwrap_or_else(|| panic!("entrée introuvable : bloc={block} item={item}"))
}

/// Référence vers une entrée par (bloc, item) ; panique si absente.
fn find<'a>(ws: &'a Workspace, block: &str, item: &str) -> &'a zoneedit::domain::Entry {
    ws.entry(find_id(ws, block, item))
}

/// Valeur éditable (champ propre) de l'entrée pour un type de champ.
fn val<'a>(ws: &'a Workspace, e: &'a zoneedit::domain::Entry, kind: FieldKind) -> &'a str {
    let id = ws.own_field(e, kind).unwrap_or_else(|| {
        panic!(
            "champ {kind:?} absent : {} / {}",
            ws.block(e.block).name,
            e.sid
        )
    });
    ws.field(id).value()
}

/// Valeur d'un champ de groupe partagé de l'entrée.
fn gval<'a>(ws: &'a Workspace, e: &'a zoneedit::domain::Entry, kind: FieldKind) -> &'a str {
    let id = e
        .group_fields
        .iter()
        .find(|&&k| ws.field(k).kind == kind)
        .copied()
        .unwrap_or_else(|| {
            panic!(
                "groupe {kind:?} absent : {} / {}",
                ws.block(e.block).name,
                e.sid
            )
        });
    ws.field(id).value()
}

fn num(s: &str) -> f64 {
    value::parse_value(s).unwrap_or_else(|| panic!("valeur non numérique : {s:?}"))
}

// ---------------------------------------------------------------------------

#[test]
fn load_sanity() {
    let ws = load_root();
    assert_eq!(ws.files().len(), 9, "9 fichiers attendus");
    let n = ws.entry_ids().count();
    assert!(n > 200, "trop peu d'entrées : {n}");
    let errs = ws.validation_errors();
    assert!(errs.is_empty(), "erreurs au chargement : {errs:?}");
}

/// Données réelles : `AnomalyPrototypes.cfg` (un bloc = une anomalie) se
/// parse sans perte (aller-retour byte-exact, nids profonds inclus) et expose
/// les champs attendus.
#[test]
fn real_anomaly_roundtrip() {
    let ws = load_root();
    let fi = ws
        .files()
        .iter()
        .position(|f| f.spec.path == "GameData/AnomalyPrototypes.cfg")
        .expect("fichier anomalies");
    let rendered = export::render(&ws, fi).expect("rendu du fichier");
    let raw = std::fs::read_to_string(ws.files()[fi].path.clone()).expect("lecture du fichier");
    assert_eq!(rendered, raw, "aller-retour non byte-exact (anomalies)");

    // Un bloc = une entrée (26 blocs dans le fichier vanilla).
    let anomalies = ws.config().category_id("Anomalies").expect("catégorie");
    let n = ws
        .entry_ids()
        .filter(|&id| ws.in_category(id, anomalies))
        .count();
    assert_eq!(n, 26, "26 blocs attendus dans le fichier vanilla");

    // Valeurs vanilla connues.
    let e = find(&ws, "ElectroAnomaly", "ElectroAnomaly");
    assert_eq!(num(val(&ws, e, FieldKind::Radius)), 200.0);
    assert_eq!(num(val(&ws, e, FieldKind::CollisionHeight)), 200.0);
    assert_eq!(num(val(&ws, e, FieldKind::Duration)), 0.1);
}

/// Données réelles : `ArtifactSpawnerPrototypes.cfg` (93 blocs, chacun avec
/// ses sous-blocs de rangs) se parse sans perte (aller-retour byte-exact)
/// et expose une entrée par bloc × rang.
#[test]
fn real_artifact_spawner_roundtrip() {
    let ws = load_root();
    let fi = ws
        .files()
        .iter()
        .position(|f| f.spec.path == "GameData/ArtifactSpawnerPrototypes.cfg")
        .expect("fichier spawners d'artefacts");
    let rendered = export::render(&ws, fi).expect("rendu du fichier");
    let raw = std::fs::read_to_string(ws.files()[fi].path.clone()).expect("lecture du fichier");
    assert_eq!(
        rendered, raw,
        "aller-retour non byte-exact (artifact spawners)"
    );

    let cat = ws
        .config()
        .category_id("ArtifactSpawner")
        .expect("catégorie");
    let n = ws.entry_ids().filter(|&id| ws.in_category(id, cat)).count();
    assert_eq!(n, 364, "une entrée par bloc × rang");

    // Valeurs vanilla connues (ChunkMeatArtifactSpawner, Newbie).
    let e = ws
        .entry_ids()
        .find(|&id| {
            let e = ws.entry(id);
            ws.block(e.block).name == "ChunkMeatArtifactSpawner"
                && e.ctx.rank_label.as_deref() == Some("Newbie")
        })
        .expect("entrée ChunkMeatArtifactSpawner/Newbie");
    let e = ws.entry(e);
    assert_eq!(num(val(&ws, e, FieldKind::ArtifactCount)), 1.0);
    assert_eq!(num(val(&ws, e, FieldKind::MinCooldown)), 3.0);
    assert_eq!(num(val(&ws, e, FieldKind::MaxCooldown)), 15.0);
    assert_eq!(num(val(&ws, e, FieldKind::SpawnChanceBase)), 40.0);
    assert_eq!(num(val(&ws, e, FieldKind::SpawnChanceBonus)), 5.0);
    assert_eq!(num(val(&ws, e, FieldKind::RarityChanceCommon)), 0.0);

    // Icon hints: the first NON-EMPTY artifact of each spawner's
    // `ListOfArtifacts`. 86 spawner blocks have both rank entries and a
    // listed artifact; the rest (no list or an empty one) have no hint. The
    // 2 quest spawners that list an artifact have no rank sub-block (they
    // inherit via refkey) so they have no entries at all.
    let blocks_with_hint = ws
        .entry_ids()
        .filter(|&id| ws.entry(id).icon_sid.is_some())
        .map(|id| ws.entry(id).block)
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(
        blocks_with_hint.len(),
        86,
        "86 spawners list a first artifact"
    );
    let razor = ws
        .entry_ids()
        .find(|&id| ws.block(ws.entry(id).block).name == "RazorArtifactSpawner")
        .expect("a RazorArtifactSpawner entry");
    assert_eq!(ws.entry(razor).icon_key(), "EArtifactRazor");
    // Quest / special spawners: the hint is still the first listed artifact
    // (which simply has no icon file — the UI falls back to the placeholder
    // at lookup). Empty lists have no hint at all.
    for (spawner, artifact) in [
        ("BrainArtifactSpawner", "PArtifactBrain"),
        ("SQ13_SoulArtifactSpawner", "SQ13_Soul"),
        ("QuestSlugArtifactSpawner", "CPrologArtifactSlug"),
    ] {
        let e = ws
            .entry_ids()
            .find(|&id| ws.block(ws.entry(id).block).name == spawner)
            .unwrap_or_else(|| panic!("{spawner} entry missing"));
        assert_eq!(ws.entry(e).icon_key(), artifact, "hint of {spawner}");
    }
    let scraper = ws
        .entry_ids()
        .find(|&id| ws.block(ws.entry(id).block).name == "ScraperArtifactSpawner")
        .expect("a ScraperArtifactSpawner entry");
    assert_eq!(ws.entry(scraper).icon_sid, None, "empty list: no hint");
    assert_eq!(ws.entry(scraper).icon_key(), "ScraperArtifactSpawner");
}

#[test]
fn medkit_experienced() {
    let ws = load_root();
    let e = find(&ws, "DestructibleStash_Medkit_Experienced", "Medkit");
    assert_eq!(num(val(&ws, e, FieldKind::Weight)), 20.0);
    assert_eq!(num(val(&ws, e, FieldKind::MinCount)), 1.0);
    // Données vanilla (ItemGeneratorPrototypes.cfg) : MaxCount = 2.
    assert_eq!(num(val(&ws, e, FieldKind::MaxCount)), 2.0);
    // Pas de champ Rank dans ce fichier : le rang est déduit du nom du
    // bloc (suffixe `_Experienced`).
    assert_eq!(
        e.ctx.label(),
        "Experienced",
        "contexte inattendu : {:?}",
        e.ctx
    );
}

#[test]
fn medkit_newbie() {
    let ws = load_root();
    let e = find(&ws, "DestructibleStash_Medkit_Newbie", "Medkit");
    assert_eq!(num(val(&ws, e, FieldKind::Weight)), 1.0);
}

#[test]
fn delegation() {
    let ws = load_root();
    let e = find(
        &ws,
        "DestructibleStash_Med_Experienced",
        "DestructibleStash_Medkit_Experienced",
    );
    assert!(e.is_delegation(), "l'entrée doit être une délégation");
    assert_eq!(num(val(&ws, e, FieldKind::Chance)), 1.0);
}

#[test]
fn all_ammos() {
    let ws = load_root();
    let e = find(&ws, "AllAmmosGenerator", "A918D");
    assert_eq!(num(val(&ws, e, FieldKind::Chance)), 1.0);
    assert_eq!(num(val(&ws, e, FieldKind::MinCount)), 900.0);
    assert_eq!(num(val(&ws, e, FieldKind::MaxCount)), 900.0);
    assert!(
        !e.fields
            .iter()
            .any(|&k| ws.field(k).kind == FieldKind::Weight),
        "Weight ne doit pas exister sur AllAmmosGenerator"
    );
}

#[test]
fn weapon_ak74() {
    let ws = load_root();
    let e = find(&ws, "PlayerItemGenerator", "GunAK74_ST");
    assert_eq!(num(val(&ws, e, FieldKind::AmmoMinCount)), 600.0);
    assert_eq!(num(val(&ws, e, FieldKind::AmmoMaxCount)), 900.0);
    assert_eq!(num(val(&ws, e, FieldKind::Weight)), 1000.0);
}

#[test]
fn smart_ammo_cheap() {
    let ws = load_root();
    let e = find(&ws, "Stash_AmmoAll_Cheap", "A918D");
    assert_eq!(num(val(&ws, e, FieldKind::MinCount)), 20.0);
    assert_eq!(num(val(&ws, e, FieldKind::MaxCount)), 20.0);
    assert_eq!(num(val(&ws, e, FieldKind::Weight)), 3.0);
    assert_eq!(num(gval(&ws, e, FieldKind::MainWeaponAmmoCount)), 30.0);
    assert_eq!(num(gval(&ws, e, FieldKind::MinSpawnChance)), 0.0);
    assert_eq!(num(gval(&ws, e, FieldKind::MaxSpawnChance)), 1.0);
    assert_eq!(e.group.as_deref(), Some("A918"));
    let label = e.ctx.label();
    assert!(label.contains("A918"), "contexte : {label:?}");
    assert!(label.contains("Newbie"), "contexte : {label:?}");
}

#[test]
fn npc_ammo_smart() {
    let ws = load_root();
    let e = find(&ws, "NPC_Ammo_Smart", "A012D");
    assert_eq!(num(val(&ws, e, FieldKind::MinCount)), 2.0);
    assert_eq!(num(val(&ws, e, FieldKind::MaxCount)), 6.0);
    assert_eq!(num(val(&ws, e, FieldKind::Weight)), 1.0);
    assert_eq!(num(gval(&ws, e, FieldKind::MainWeaponAmmoCount)), 12.0);
    assert_eq!(num(gval(&ws, e, FieldKind::MinSpawnChance)), 0.1);
    assert_eq!(num(gval(&ws, e, FieldKind::MaxSpawnChance)), 0.8);
    assert!(
        e.ctx.label().contains("A012"),
        "contexte : {:?}",
        e.ctx.label()
    );
}

#[test]
fn dynamic_weapon() {
    let ws = load_root();
    let e = find(&ws, "DynamicTraderItemGenerator", "GunMark_SP");
    assert_eq!(num(val(&ws, e, FieldKind::AmmoMinCount)), 600.0);
    assert_eq!(num(val(&ws, e, FieldKind::AmmoMaxCount)), 900.0);
    assert!(
        e.ctx.label().contains("Newbie"),
        "contexte : {:?}",
        e.ctx.label()
    );
}

/// Données réelles : la coquille du jeu `AmmoMaxcount` (minuscule, 4 lignes
/// dans les loadouts NPC) est lue comme `AmmoMaxCount`.
#[test]
fn real_ammo_maxcount_misspelling() {
    let ws = load_root();
    let grim = find(
        &ws,
        "GuardNPC_Monolith_Stormtrooper_ItemGenerator",
        "GuardGunGrim_ST",
    );
    assert_eq!(num(val(&ws, grim, FieldKind::AmmoMaxCount)), 45.0);
    let ak74 = find(&ws, "Solovei_ItemGenerator", "GunAK74_ST");
    assert_eq!(num(val(&ws, ak74, FieldKind::AmmoMaxCount)), 12.0);
}

#[test]
fn stash_medicine_cheap() {
    let ws = load_root();
    let e = find(&ws, "StashMedicine_Cheap", "Medkit");
    assert_eq!(num(val(&ws, e, FieldKind::MinCount)), 3.0);
    assert_eq!(num(val(&ws, e, FieldKind::MaxCount)), 5.0);
    assert_eq!(num(val(&ws, e, FieldKind::Weight)), 5.0);
    assert_eq!(num(gval(&ws, e, FieldKind::MinSpawnChance)), 0.0);
    assert_eq!(num(gval(&ws, e, FieldKind::MaxSpawnChance)), 1.0);
    assert_eq!(e.group.as_deref(), Some("Newbie")); // regroupe par rank (cf. StashMedicine_Cheap)
}

// ---------------------------------------------------------------------------

#[test]
fn new_categories() {
    let ws = load_root();
    let cfg = ws.config();
    let counts = ws.category_counts(None);
    let n = |id: &str| {
        let cid = cfg.category_id(id).expect("catégorie inconnue");
        counts
            .iter()
            .find(|(c, _)| *c == cid)
            .map(|(_, n)| *n)
            .unwrap_or(0)
    };
    // Chaque famille doit capter des entrées sur les données vanilla.
    for id in [
        "Food",
        "Grenade",
        "Armor",
        "Artifact",
        "Loot",
        "Attach",
        "Detectors",
        "Quest",
        "NpcLoadout",
    ] {
        assert!(n(id) > 0, "catégorie vide : {id} (counts: {counts:?})");
    }
    // Charge des NPCs : les blocs de faction de DynamicItemGenerator sont
    // regroupés (armes ET armures au même endroit), délégations exclues.
    assert!(
        n("NpcLoadout") > 300,
        "NpcLoadout trop vide : {}",
        n("NpcLoadout")
    );
    let npc = cfg.category_id("NpcLoadout").expect("catégorie NpcLoadout");
    let npc_weapon = ws
        .entry_ids()
        .find(|&id| {
            ws.block(ws.entry(id).block).name == "GeneralNPC_Duty_Stormtrooper_ItemGenerator"
        })
        .expect("bloc GeneralNPC_Duty_Stormtrooper_ItemGenerator absent");
    assert!(ws.in_category(npc_weapon, npc));
    let npc_armor = ws
        .entry_ids()
        .find(|&id| ws.block(ws.entry(id).block).name == "GeneralNPC_Duty_Armor")
        .expect("bloc GeneralNPC_Duty_Armor absent");
    assert!(ws.in_category(npc_armor, npc));
    assert!(
        !ws.entry_ids().any(|id| {
            let e = ws.entry(id);
            ws.block(e.block).name.starts_with("GeneralNPC")
                && e.file == 1
                && e.is_delegation()
                && ws.in_category(id, npc)
        }),
        "délégations de blocs NPC exposées à tort"
    );
    // Spot checks de classification par nom d'item, sur le fichier 0
    // (ItemGeneratorPrototypes.cfg) : les blocs de charge NPC du fichier 1
    // sont regroupés en NpcLoadout et voleraient le premier match.
    let by_item = |item: &str| {
        let id = ws
            .entry_ids()
            .find(|&id| {
                let e = ws.entry(id);
                e.file == 0 && e.sid == item
            })
            .unwrap_or_else(|| panic!("item introuvable dans le fichier 0 : {item}"));
        ws.categories_of(id).first().copied()
    };
    assert_eq!(by_item("Sausage"), cfg.category_id("Food"));
    assert_eq!(by_item("GrenadeF1"), cfg.category_id("Grenade"));
    assert_eq!(by_item("Battle_Varta_Armor"), cfg.category_id("Armor"));
    assert_eq!(by_item("CArtifactSlug"), cfg.category_id("Artifact"));
    assert_eq!(by_item("DeerLoot"), cfg.category_id("Loot"));
    assert_eq!(by_item("RU_Silen_1"), cfg.category_id("Attach"));
    // Anomaly detectors go to the Detectors category (neither Mutant Loot
    // nor Armor, even when they look like loot items).
    assert_eq!(by_item("Gilka"), cfg.category_id("Detectors"));
    let detectors = cfg.category_id("Detectors").expect("Detectors category");
    assert!(
        ws.entry_ids()
            .filter(|&id| ws.entry(id).sid == "Bear")
            // A Bear sold by a trader belongs to the TRADERS page, not
            // Detectors (exclude_block on the family rules).
            .filter(|&id| !ws
                .block(ws.entry(id).block)
                .name
                .contains("TradeItemGenerator"))
            .all(|id| ws.in_category(id, detectors)),
        "Bear must be classified Detectors wherever it appears outside trade blocks"
    );
    assert_eq!(
        by_item("Blueprint_Zubr_Upgrade_1"),
        cfg.category_id("Quest")
    );
}

/// Real data: every trader-named block (name contains "Trade"/"trader")
/// is classified in the TRADERS section and no trader row leaks into any
/// other section.
#[test]
fn real_traders_page() {
    let ws = load_root();
    let cfg = ws.config();

    let trader_cats: Vec<CategoryId> = (0..cfg.categories.len())
        .map(|i| CategoryId(i as u16))
        .filter(|&cid| cfg.categories[cid.idx()].section == "TRADERS")
        .collect();
    assert_eq!(trader_cats.len(), 21, "21 TRADERS categories");

    // Every trader-named block ("Trade"/"trader" in the name) lives on the
    // TRADERS page: all of its rows are classified there, nowhere else.
    let mut blocks: Vec<&str> = Vec::new();
    let mut rows = 0u32;
    for id in ws.entry_ids() {
        let e = ws.entry(id);
        let b = ws.block(e.block).name.as_str();
        if !b.contains("Trade") && !b.contains("trader") {
            continue;
        }
        rows += 1;
        if !blocks.contains(&b) {
            blocks.push(b);
        }
        let cats = ws.categories_of(id);
        assert!(!cats.is_empty(), "trader row unclassified: {b} / {}", e.sid);
        assert!(
            cats.iter().all(|&c| trader_cats.contains(&c)),
            "trader row leaked into a non-TRADERS category: {b} / {}",
            e.sid
        );
    }
    assert_eq!(
        blocks.len(),
        97,
        "all trader-named blocks of the loaded files"
    );
    // Item + delegation scopes across the 97 blocks (64 trade blocks:
    // 368 items + 166 delegations, plus the sub-pool / wandering-trader
    // generators).
    assert_eq!(rows, 847, "trader rows (items + delegations)");
}

// ---------------------------------------------------------------------------
// Fixtures : tests du cœur (cfg / domain / io) sans les fichiers du jeu
// ---------------------------------------------------------------------------

/// Racine des fixtures : `tests/fixtures/` contient `GameData/...`.
fn fixture_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
}

fn load_fixtures() -> Workspace {
    load::load(&fixture_root()).expect("chargement des fixtures")
}

/// Aller-retour : sans modification, `render` redonne EXACTEMENT les
/// octets d'origine (BOM, CRLF/LF, retour final absent ou présent).
#[test]
fn fixture_roundtrip_is_byte_exact() {
    let ws = load_fixtures();
    assert_eq!(ws.files().len(), 9, "9 fichiers attendus dans les fixtures");
    for (i, f) in ws.files().iter().enumerate() {
        let rendered = export::render(&ws, i).unwrap_or_else(|e| panic!("{} : {e}", f.name()));
        let raw = std::fs::read(&f.path).unwrap_or_else(|e| panic!("{} : {e}", f.name()));
        assert_eq!(
            rendered.as_bytes(),
            &raw[..],
            "aller-retour non byte-exact pour {}",
            f.name()
        );
    }
}

/// Anomalies (`AnomalyPrototypes.cfg`, rôle `anomalies`) : un bloc = une
/// entrée, champs à plat du bloc, classés dans la catégorie `Anomalies`.
#[test]
fn fixture_anomalies_block_as_entry() {
    let ws = load_fixtures();
    let anomalies = ws
        .config()
        .category_id("Anomalies")
        .expect("catégorie Anomalies");

    // Une entrée par bloc de la fixture (6 blocs).
    let ids: Vec<_> = ws
        .entry_ids()
        .filter(|&id| ws.in_category(id, anomalies))
        .collect();
    assert_eq!(ids.len(), 6, "un bloc = une entrée anomalie");

    // ElectroAnomaly : champs à plat, SID = nom du bloc.
    let e = find(&ws, "ElectroAnomaly", "ElectroAnomaly");
    assert_eq!(num(val(&ws, e, FieldKind::Radius)), 200.0);
    assert_eq!(num(val(&ws, e, FieldKind::Cooldown)), 3.0);
    assert_eq!(num(val(&ws, e, FieldKind::Duration)), 0.1);
    assert_eq!(num(val(&ws, e, FieldKind::CollisionHeight)), 200.0);
    assert!(
        ws.own_field(e, FieldKind::ActivationSpeedThreshold)
            .is_none(),
        "champ absent non inventé"
    );

    // ClassicFireAnomaly : valeurs entières acceptées pour un champ float.
    let fire = find(&ws, "ClassicFireAnomaly", "ClassicFireAnomaly");
    assert_eq!(num(val(&ws, fire, FieldKind::Cooldown)), 5.0);
    assert_eq!(num(val(&ws, fire, FieldKind::Duration)), 3.0);

    // ElectroAnomaly_Dynamic: child variant (refkey=ElectroAnomaly) — its own
    // fields (inherited + specific) are extracted as the entry's own fields.
    let dynamic = find(&ws, "ElectroAnomaly_Dynamic", "ElectroAnomaly_Dynamic");
    assert_eq!(
        num(val(&ws, dynamic, FieldKind::ActivationSpeedThreshold)),
        40.0
    );
    assert_eq!(
        num(val(&ws, dynamic, FieldKind::MovementLocationPriorityRadius)),
        3500.0
    );
    assert_eq!(num(val(&ws, dynamic, FieldKind::MovementSpeed)), 300.0);
    assert!(
        ws.own_field(dynamic, FieldKind::Radius).is_none(),
        "inherited Radius is not re-declared on the variant"
    );

    // Bloc sans champ éditable (hérite tout) : entrée vide, pas d'erreur.
    let psy = find(&ws, "PSYControllerAnomaly", "PSYControllerAnomaly");
    assert!(psy.fields.is_empty(), "aucun champ propre");

    // Pas de champ égaré : `Radius` n'apparaît que dans les anomalies et
    // les spawners d'artefacts du monde.
    let spawners = ws
        .config()
        .category_id("ArtifactSpawner")
        .expect("catégorie ArtifactSpawner");
    for id in ws.entry_ids() {
        let e = ws.entry(id);
        if !ws.in_category(id, anomalies) && !ws.in_category(id, spawners) {
            assert!(
                ws.own_field(e, FieldKind::Radius).is_none(),
                "Radius hors anomalies : {} / {}",
                ws.block(e.block).name,
                e.sid
            );
        }
    }
}

/// Export d'une anomalie : patch prototype dans `AnomalyPrototypes/`,
/// nœud `{bpatch}` adressé par le nom du bloc, seul le champ modifié.
#[test]
fn fixture_anomaly_export_patch() {
    let mut ws = load_fixtures();
    let fi = ws
        .files()
        .iter()
        .position(|f| f.spec.path == "GameData/AnomalyPrototypes.cfg")
        .expect("fichier anomalies");
    let e = find(&ws, "ElectroAnomaly", "ElectroAnomaly");
    let k = ws.own_field(e, FieldKind::Radius).expect("Radius");
    ws.set_text(k, "250.0");
    let patch = export::render_patch(&ws, fi).expect("rendu du patch");
    assert_eq!(
        patch,
        concat!(
            "ElectroAnomaly : struct.begin {bpatch}\n",
            "    Radius = 250.0\n",
            "struct.end\n"
        )
    );
    assert_eq!(
        export::patch_rel_path("GameData/AnomalyPrototypes.cfg", "lololo"),
        std::path::PathBuf::from("GameData/AnomalyPrototypes/lololo.cfg")
    );
}

/// Spawners d'artefacts du monde (`ArtifactSpawnerPrototypes.cfg`) : une
/// entrée par bloc × rang. L'export est un patch `{bpatch}` adressé par le
/// nom du bloc ; les rangs sont des nœuds imbriqués, `RarityChance` plus
/// loin encore ; les clés émisses sont celles du fichier de base
/// (`Count`, `Common`, pas les clés canoniques des aliases).
#[test]
fn fixture_artifact_spawner_export_patch() {
    let mut ws = load_fixtures();
    let fi = ws
        .files()
        .iter()
        .position(|f| f.spec.path == "GameData/ArtifactSpawnerPrototypes.cfg")
        .expect("fichier spawners d'artefacts");

    // ChunkMeatArtifactSpawner : une entrée par rang (Newbie + Veteran).
    let spawners: Vec<_> = ws
        .entry_ids()
        .filter(|&id| ws.block(ws.entry(id).block).name == "ChunkMeatArtifactSpawner")
        .collect();
    assert_eq!(spawners.len(), 2, "une entrée par rang");
    let by_rank = |rank: &str| {
        spawners
            .iter()
            .find(|&&id| ws.entry(id).ctx.rank_label.as_deref() == Some(rank))
            .copied()
            .unwrap_or_else(|| panic!("entrée {rank} absente"))
    };
    let newbie = by_rank("Newbie");
    let veteran = by_rank("Veteran");

    // Valeurs de la fixture.
    let k_count0 = ws
        .own_field(ws.entry(newbie), FieldKind::ArtifactCount)
        .expect("Count absent");
    assert_eq!(ws.field(k_count0).original(), "4");

    // Icon hint: the first artifact of `ListOfArtifacts` (shared by both
    // rank entries of the block; no list → no hint).
    for &id in &spawners {
        assert_eq!(
            ws.entry(id).icon_key(),
            "CArtifactChunkMeat",
            "indice d'icône du spawner"
        );
    }
    let empty = ws
        .entry_ids()
        .find(|&id| ws.block(ws.entry(id).block).name == "Empty")
        .expect("bloc de base Empty");
    assert_eq!(ws.entry(empty).icon_key(), "Empty");

    // Newbie : Count ; Veteran : la `Common` du sous-scope RarityChance.
    ws.set_text(k_count0, "7");
    let k_common = ws
        .own_field(ws.entry(veteran), FieldKind::RarityChanceCommon)
        .expect("Common absent");
    ws.set_text(k_common, "45.f");

    let patch = export::render_patch(&ws, fi).expect("rendu du patch");
    assert_eq!(
        patch,
        concat!(
            "ChunkMeatArtifactSpawner : struct.begin {bpatch}\n",
            "    Newbie : struct.begin\n",
            "        Count = 7\n",
            "    struct.end\n",
            "    Veteran : struct.begin\n",
            "        RarityChance : struct.begin\n",
            "            Common = 45.f\n",
            "        struct.end\n",
            "    struct.end\n",
            "struct.end\n"
        )
    );
}

/// Modifier une valeur → seule cette ligne diffère, format préservé.
#[test]
fn fixture_single_line_change() {
    let mut ws = load_fixtures();
    let e = find(&ws, "DestructibleStash_Medkit_Newbie", "Medkit");
    let k = ws.own_field(e, FieldKind::Weight).expect("Weight absent");
    let line = ws.field(k).line;
    ws.set_text(k, "7");
    let rendered = export::render(&ws, 0).unwrap();
    let raw = std::fs::read_to_string(&ws.files()[0].path).unwrap();
    let r: Vec<&str> = rendered.lines().collect();
    let o: Vec<&str> = raw.lines().collect();
    assert_eq!(r.len(), o.len(), "même nombre de lignes");
    let diff: Vec<usize> = (0..r.len()).filter(|i| r[*i] != o[*i]).collect();
    assert_eq!(diff, vec![line - 1], "seule la ligne modifiée diffère");
    assert!(
        r[line - 1].contains("Weight = 7"),
        "ligne : {}",
        r[line - 1]
    );
}

/// Structure extraite sur les fixtures : délégation, `empty`, rang composé,
/// itemsets (« Set N · item »), calibres, champs de groupe partagés.
#[test]
fn fixture_parser_structure() {
    let ws = load_fixtures();
    assert!(ws.struct_balanced(), "struct.begin / struct.end équilibrés");

    // Délégation.
    let d = find(
        &ws,
        "DestructibleStash_Med_Experienced",
        "DestructibleStash_Medkit_Experienced",
    );
    assert!(d.is_delegation(), "entrée de délégation");
    assert!(
        !ws.entry_ids().any(|id| {
            let e = ws.entry(id);
            ws.block(e.block).name == "DestructibleStash_Med_Experienced" && !e.is_delegation()
        }),
        "aucune entrée non délégation dans le bloc délégation"
    );

    // `empty` n'ouvre pas d'entrée (fichier 0 et QuestItemGenerator).
    assert!(!ws.entry_ids().any(|id| ws.entry(id).sid == "empty"));
    assert!(
        !ws.entry_ids().any(|id| ws.entry(id).file == 2),
        "aucune entrée dans le fichier Quest (tout est empty)"
    );

    // Rang composé « Veteran, Master » (DynamicItemGenerator).
    let t = find(&ws, "DynamicTraderItemGenerator", "GunMark_SP");
    assert_eq!(
        t.ctx.rank_label.as_deref(),
        Some("Veteran, Master"),
        "rang composé : {:?}",
        t.ctx
    );

    // Itemsets : libellés positionnels + regroupement par rang.
    let m = find(&ws, "StashMedicine_Cheap", "Medkit");
    assert_eq!(m.itemset_label.as_deref(), Some("Set 1 · Medkit"));
    assert_eq!(m.group.as_deref(), Some("Newbie"));
    let b = find(&ws, "StashMedicine_Cheap", "Bandage");
    assert_eq!(b.itemset_label.as_deref(), Some("Set 2 · Bandage"));
    // Le groupe (MinSpawnChance/MaxSpawnChance) est partagé : une seule
    // instance par (fichier, ligne, champ), référencée par les deux entrées.
    let gm = m
        .group_fields
        .iter()
        .find(|&&k| ws.field(k).kind == FieldKind::MinSpawnChance)
        .copied()
        .unwrap();
    let gb = b
        .group_fields
        .iter()
        .find(|&&k| ws.field(k).kind == FieldKind::MinSpawnChance)
        .copied()
        .unwrap();
    assert_ne!(gm, gb, "sets différents → champs de groupe différents");
    let same: Vec<EntryId> = ws
        .entry_ids()
        .filter(|&id| {
            let e = ws.entry(id);
            ws.block(e.block).name == "StashMedicine_Smart" && e.sid == "Medkit"
        })
        .collect();
    assert_eq!(same.len(), 2, "StashMedicine_Smart : Medkit dans 2 rangs");

    // Calibre : groupe MainAmmo/MinSp/MaxSp par calibre, contexte rang+calibre.
    let a = find(&ws, "Stash_AmmoAll_Cheap", "A918D");
    assert_eq!(a.group.as_deref(), Some("A918"));
    let label = a.ctx.label();
    assert!(
        label.contains("Newbie") && label.contains("A918"),
        "{label:?}"
    );
    let a045 = find(&ws, "Stash_AmmoAll_Cheap", "A045D");
    let ka918 = a
        .group_fields
        .iter()
        .find(|&&k| ws.field(k).kind == FieldKind::MainWeaponAmmoCount)
        .copied()
        .unwrap();
    let ka045 = a045
        .group_fields
        .iter()
        .find(|&&k| ws.field(k).kind == FieldKind::MainWeaponAmmoCount)
        .copied()
        .unwrap();
    assert_ne!(
        ka918, ka045,
        "calibres différents → champs de groupe différents"
    );
}

/// `classify` (règles de la configuration) sur les familles types des fixtures.
#[test]
fn fixture_classify() {
    let ws = load_fixtures();
    let cfg = ws.config();
    let c = |block: &str, item: &str| ws.categories_of(find_id(&ws, block, item)).first().copied();
    assert_eq!(
        c("DestructibleStash_Medkit_Newbie", "Medkit"),
        cfg.category_id("MedCrate")
    );
    assert_eq!(
        c(
            "DestructibleStash_Med_Experienced",
            "DestructibleStash_Medkit_Experienced"
        ),
        cfg.category_id("MedCrate"),
        "délégation dans un bloc de caisse médicale exposée"
    );
    assert_eq!(
        c("StashMedicine_Cheap", "Medkit"),
        cfg.category_id("MedStash")
    );
    assert_eq!(c("StashFood_Cheap", "Vodka"), cfg.category_id("Food"));
    assert_eq!(
        c("AllAmmosGenerator", "A918D"),
        cfg.category_id("AmmoGenerator")
    );
    assert_eq!(
        c("Stash_AmmoAll_Cheap", "A918D"),
        cfg.category_id("AmmoSmart")
    );
    assert_eq!(
        c("PlayerItemGenerator", "GunAK74_ST"),
        cfg.category_id("AmmoWeapon")
    );
    assert_eq!(
        c("PlayerItemGenerator", "Medkit"),
        cfg.category_id("MedDirect")
    );
    assert_eq!(
        c("GeneralNPC_Duty_Stormtrooper_ItemGenerator", "GunAK74_ST"),
        cfg.category_id("NpcLoadout")
    );
    assert_eq!(
        c("GeneralNPC_Duty_Armor", "Battle_Varta_Armor"),
        cfg.category_id("NpcLoadout")
    );
    // Trader-named blocks are excluded from the family categories and
    // classified on the TRADERS page (Universal / Other).
    assert_eq!(
        c("DynamicTraderItemGenerator", "GunMark_SP"),
        cfg.category_id("UniversalOther")
    );
}

/// Traders page: `*_TradeItemGenerator` blocks are classified by location
/// (TRADERS section); their items are NOT captured by the family
/// categories (`exclude_block`); the delegation references are shown on
/// the page (`delegations = "include"`).
#[test]
fn fixture_traders() {
    let ws = load_fixtures();
    let cfg = ws.config();
    let sultansk = cfg.category_id("Sultansk").expect("Sultansk category");
    let universal = cfg
        .category_id("UniversalOther")
        .expect("UniversalOther category");
    let armor = cfg.category_id("Armor").expect("Armor category");
    let food = cfg.category_id("Food").expect("Food category");
    let grenade = cfg.category_id("Grenade").expect("Grenade category");
    let npc = cfg.category_id("NpcLoadout").expect("NpcLoadout category");

    // Every row of the Sultansk trader block (3 items + 1 delegation)
    // belongs to the Sultansk category.
    let block_ids: Vec<_> = ws
        .entry_ids()
        .filter(|&id| ws.block(ws.entry(id).block).name == "SultanskTrader_TradeItemGenerator")
        .collect();
    assert_eq!(block_ids.len(), 4, "3 items + 1 delegation");
    for &id in &block_ids {
        assert!(ws.in_category(id, sultansk), "trade row outside Sultansk");
    }

    // exclude_block regression: items sold by a trader are not captured
    // by their family categories.
    let armor_id = find_id(
        &ws,
        "SultanskTrader_TradeItemGenerator",
        "Battle_Varta_Armor",
    );
    assert!(
        !ws.in_category(armor_id, armor),
        "trade armor leaked into Armor"
    );
    let grenade_id = find_id(&ws, "SultanskTrader_TradeItemGenerator", "GrenadeF1");
    assert!(
        !ws.in_category(grenade_id, grenade),
        "trade grenade leaked into Grenade"
    );

    // Delegation (↳ MedicUniversal): shown on the Sultansk page, while
    // the target block's own items live in UniversalOther.
    let deleg = block_ids
        .iter()
        .find(|&&id| ws.entry(id).kind == EntryKind::Delegation)
        .copied()
        .expect("delegation row in the Sultansk block");
    assert_eq!(ws.entry(deleg).sid, "MedicUniversal_TradeItemGenerator");
    let medkit = find_id(&ws, "MedicUniversal_TradeItemGenerator", "Medkit");
    assert_eq!(
        ws.categories_of(medkit).first().copied(),
        Some(universal),
        "delegation target block classified UniversalOther"
    );

    // Shared trader sub-pool (name contains "Trader"): excluded from its
    // item family (Food) and classified UniversalOther.
    let bread = find_id(&ws, "Trader_Food_Fixture_ItemGenerator", "Bread");
    assert!(!ws.in_category(bread, food), "trader pool leaked into Food");
    assert!(
        ws.in_category(bread, universal),
        "trader pool not in UniversalOther"
    );

    // Non-trade faction block: unaffected, still NpcLoadout.
    let ak = find_id(
        &ws,
        "GeneralNPC_Duty_Stormtrooper_ItemGenerator",
        "GunAK74_ST",
    );
    assert_eq!(ws.categories_of(ak).first().copied(), Some(npc));
}

/// Validation : nombre, ≥ 0, ≤ 1 pour les spawn chances, entier pour les
/// counts ; Min ≤ Max seulement si la paire est touchée par l'utilisateur.
#[test]
fn fixture_validation() {
    let ws0 = load_fixtures();
    let cfg = ws0.config();
    let ck = |k: FieldKind, s: &str| value::check(cfg.spec(k), s).is_ok();
    // `check` pur.
    assert!(ck(FieldKind::Weight, "10"));
    assert!(ck(FieldKind::Weight, "100.0"));
    assert!(ck(FieldKind::Weight, "0.f"));
    assert!(ck(FieldKind::Weight, "0.29f"));
    assert!(!ck(FieldKind::Weight, "abc"), "non numérique");
    assert!(!ck(FieldKind::Weight, "-1"), "négatif");
    assert!(ck(FieldKind::MinSpawnChance, "0"));
    assert!(ck(FieldKind::MinSpawnChance, "1"));
    assert!(!ck(FieldKind::MinSpawnChance, "1.5"), "spawn chance > 1");
    assert!(!ck(FieldKind::MaxCount, "2.5"), "count non entier");
    assert!(ck(FieldKind::MaxCount, "0"));

    // Vanilla non touché → aucune erreur (les fixtures comportent un
    // Min = 25 > Max = 15 sur A762NATOD).
    let ws = load_fixtures();
    assert!(ws.validation_errors().is_empty(), "erreurs au chargement");

    // L'utilisateur touche la paire → erreur Min > Max.
    let mut ws = load_fixtures();
    let e = find(&ws, "CommonCacheAmmoGenerator", "A762NATOD");
    let kmin = ws.own_field(e, FieldKind::MinCount).unwrap();
    ws.set_text(kmin, "30");
    let errs = ws.validation_errors();
    assert!(
        errs.iter()
            .any(|v| v.to_string().contains("Min (30) > Max (15)")),
        "erreur Min > Max attendue : {errs:?}"
    );

    // Min ≤ Max après correction → plus d'erreur.
    ws.set_text(kmin, "10");
    assert!(ws.validation_errors().is_empty(), "corrigé");

    // Champ non numérique dirty → erreur de validation.
    let mut ws = load_fixtures();
    let e = find(&ws, "AllAmmosGenerator", "A918D");
    let kmax = ws.own_field(e, FieldKind::MaxCount).unwrap();
    ws.set_text(kmax, "beaucoup");
    assert_eq!(ws.validation_errors().len(), 1);
}

/// Recopie les fixtures dans un répertoire temporaire (les tests tournent
/// en parallèle : on ne doit JAMAIS modifier les fixtures partagées).
fn copy_fixtures_to_temp(tag: &str) -> (PathBuf, PathBuf) {
    let src = fixture_root();
    let dst = std::env::temp_dir().join(format!("zoneedit_fixture_{tag}"));
    let _ = std::fs::remove_dir_all(&dst);
    fn copy_rec(s: &Path, d: &Path) {
        for p in std::fs::read_dir(s).unwrap() {
            let p = p.unwrap().path();
            let d2 = d.join(p.file_name().unwrap());
            if p.is_dir() {
                std::fs::create_dir_all(&d2).unwrap();
                copy_rec(&p, &d2);
            } else {
                std::fs::copy(&p, &d2).unwrap();
            }
        }
    }
    std::fs::create_dir_all(&dst).unwrap();
    copy_rec(&src.join("GameData"), &dst.join("GameData"));
    let out = dst.join("out");
    (dst, out)
}

/// Export integrity: a source file modified during the session is
/// rejected; the original is NEVER written.
#[test]
fn fixture_export_rejects_modified_source() {
    let (root, out) = copy_fixtures_to_temp("reject");
    let mut ws = load::load(&root).expect("load");
    let before = std::fs::read(&ws.files()[0].path).expect("read before edit");
    // A dirty field so the export writes something.
    let e = find(&ws, "DestructibleStash_Medkit_Newbie", "Medkit");
    let k = ws.own_field(e, FieldKind::Weight).unwrap();
    ws.set_text(k, "9");

    let written = export::export(&ws, &out, "zoneedit").expect("fixture export");
    assert_eq!(written.len(), 1, "one modified file exported");
    assert_eq!(written[0].name, "ItemGeneratorPrototypes");
    assert!(
        written[0]
            .path
            .ends_with("GameData/ItemGeneratorPrototypes/zoneedit.cfg"),
        "patch path: {:?}",
        written[0].path
    );

    // The original is intact (byte-exact after the export).
    let raw = std::fs::read(&ws.files()[0].path).expect("read after export");
    assert_eq!(raw, before, "the original is never written");

    // Source file size modified during the session → refused.
    let quest = ws.files()[2].path.clone();
    let _quest_bytes = std::fs::read(&quest).unwrap();
    std::fs::write(&quest, [0u8]).unwrap();
    let err = export::export(&ws, &out, "zoneedit").expect_err("export must fail");
    assert!(
        err.to_string().contains("modified during the session"),
        "{err}"
    );

    let _ = std::fs::remove_dir_all(&root);
}

/// Export: one patch file per modified base file, placed in the prototype
/// subfolder (`out/GameData/<Base>/<mod>.cfg`); the other files of the
/// mod (a patch written by another mod, an old mirror copy) are never
/// touched.
#[test]
fn fixture_export_patch_tree() {
    let (root, out) = copy_fixtures_to_temp("mirror");
    let mut ws = load::load(&root).expect("load");

    // One dirty field per file (0 and 3).
    let e = find(&ws, "DestructibleStash_Medkit_Newbie", "Medkit");
    let k = ws.own_field(e, FieldKind::Weight).unwrap();
    ws.set_text(k, "2");
    let ks = ws
        .field_ids()
        .find(|&id| ws.field(id).file == 3 && ws.field(id).kind == FieldKind::MinSpawnChance)
        .expect("a MinSpawnChance in StashPrototypes");
    ws.set_text(ks, "0.5f");

    let written = export::export(&ws, &out, "zoneedit").expect("export");
    let paths: Vec<String> = written
        .iter()
        .map(|x| x.path.display().to_string())
        .collect();
    assert!(
        paths
            .iter()
            .any(|p| p.ends_with("GameData/ItemGeneratorPrototypes/zoneedit.cfg")),
        "file 0: {paths:?}"
    );
    assert!(
        paths
            .iter()
            .any(|p| p.ends_with("GameData/StashPrototypes/zoneedit.cfg")),
        "file 3: {paths:?}"
    );
    // File 2 (not modified): no patch for it.
    assert!(
        !paths
            .iter()
            .any(|p| p.contains("QuestItemGeneratorPrototypes/")),
        "unmodified file 2: {paths:?}"
    );
    // No .cfg at the root of the export.
    assert!(
        !std::fs::read_dir(&out).unwrap().any(|e| e
            .unwrap()
            .path()
            .extension()
            .and_then(|x| x.to_str())
            == Some("cfg")),
        "no .cfg file at the root of the export"
    );

    // The mod folder is user-managed: a re-export overwrites only the own
    // patch and leaves the other files untouched.
    let mirror = out.join("GameData/ItemGeneratorPrototypes.cfg");
    std::fs::write(&mirror, "obsolete").unwrap();
    let other = out.join("GameData/StashPrototypes/othermod.cfg");
    std::fs::create_dir_all(other.parent().unwrap()).unwrap();
    std::fs::write(&other, "obsolete").unwrap();
    export::export(&ws, &out, "zoneedit").expect("re-export");
    assert!(mirror.exists(), "foreign file must not be removed");
    assert!(other.exists(), "patch of another mod must not be removed");
    assert!(
        out.join("GameData/StashPrototypes/zoneedit.cfg").exists(),
        "own patch overwritten on re-export"
    );

    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn export_roundtrip() {
    let mut ws = load_root();
    let before = std::fs::read(&ws.files()[0].path).expect("read the original");
    let e = find(&ws, "DestructibleStash_Medkit_Experienced", "Medkit");
    let key = ws
        .own_field(e, FieldKind::Weight)
        .expect("Weight missing on Medkit (Experienced)");
    ws.set_text(key, "30");
    assert_eq!(ws.dirty_count(), 1, "one modification expected");

    let out_dir = std::env::temp_dir().join("zoneedit_test_export");
    let _ = std::fs::remove_dir_all(&out_dir);
    let written = export::export(&ws, &out_dir, "zoneedit").expect("export failed");
    assert_eq!(written.len(), 1, "one file exported");
    assert_eq!(written[0].name, "ItemGeneratorPrototypes");
    assert!(
        written[0]
            .path
            .ends_with("GameData/ItemGeneratorPrototypes/zoneedit.cfg"),
        "patch path: {:?}",
        written[0].path
    );

    let raw = std::fs::read_to_string(&written[0].path).expect("cannot read the export");
    // The modified field is re-emitted at its real depth: the block plus
    // the four scopes in between, and nothing else. The `{bpatch}` keyword
    // is carried only by the root (the merge is recursive).
    assert!(
        raw.starts_with("DestructibleStash_Medkit_Experienced : struct.begin {bpatch}\n"),
        "exported patch: {raw}"
    );
    assert!(
        raw.contains("    ItemGenerator : struct.begin\n"),
        "nested scopes are plain: {raw}"
    );
    assert!(
        raw.contains("        [0] : struct.begin\n"),
        "nested scopes are plain: {raw}"
    );
    assert!(raw.contains("Weight = 30"), "exported patch: {raw}");
    assert_eq!(
        raw.matches("struct.begin {bpatch}").count(),
        1,
        "only the root carries {{bpatch}}: {raw}"
    );
    assert_eq!(
        raw.matches("struct.begin").count(),
        5,
        "block + 4 nested scopes: {raw}"
    );
    assert_eq!(raw.matches("struct.end").count(), 5, "balanced struct.end");

    // The original is intact (byte-exact after the export).
    let after = std::fs::read(&ws.files()[0].path).expect("read after export");
    assert_eq!(after, before, "the original is never written");
}

/// Presets v3 (TOML) : save/apply par contexte, vérification de la valeur
/// d'origine, doublon sur le slug, lecture des formats v1 (légués).
#[test]
fn fixture_preset_v3() {
    let (root, _out) = copy_fixtures_to_temp("preset");
    let mut ws = load::load(&root).expect("chargement");

    // Deux champs dirty : Weight (fichier 0) + MinSpawnChance (fichier 3).
    let e = find(&ws, "DestructibleStash_Medkit_Newbie", "Medkit");
    let k = ws.own_field(e, FieldKind::Weight).unwrap();
    ws.set_text(k, "3");
    let ks = ws
        .field_ids()
        .find(|&id| ws.field(id).file == 3 && ws.field(id).kind == FieldKind::MinSpawnChance)
        .expect("un MinSpawnChance dans StashPrototypes");
    ws.set_text(ks, "0.25f");

    let path = presets::save(&ws, "Test  Preset").expect("save");
    // Slug : minuscules, séquences de `_` réduites.
    assert_eq!(
        path.file_name().unwrap().to_string_lossy(),
        "test_preset.preset"
    );
    let text = std::fs::read_to_string(&path).expect("lecture");
    assert!(
        text.starts_with("# zoneEDIT preset (format 3)\n"),
        "en-tête : {text:?}"
    );

    // Doublon sur le slug (nom différent, même slug).
    assert!(
        presets::save(&ws, "Test-Preset").is_err(),
        "doublon de slug refusé"
    );

    // Chargement + application sur un workspace neuf.
    let (list, errs) = presets::load_all(&root, ws.config());
    assert!(errs.is_empty(), "presets illisibles : {errs:?}");
    assert_eq!(list.len(), 1, "un seul preset");
    assert_eq!(list[0].version, 3, "preset v3");
    assert_eq!(list[0].fields.len(), 2);
    let mut ws2 = load::load(&root).expect("chargement 2");
    let rep = presets::apply(&mut ws2, &list[0]);
    assert_eq!(
        (rep.applied, rep.skipped),
        (2, 0),
        "deux appliqués, zéro ignorés"
    );
    assert_eq!(ws2.field(k).value(), "3");
    assert_eq!(ws2.field(ks).value(), "0.25f");

    // Valeur d'origine changée (patch du jeu) → champ ignoré.
    let mut p = list[0].clone();
    if let Some(pf) = p.fields.iter_mut().find(|f| f.kind == FieldKind::Weight) {
        pf.original = Some("999".into());
    }
    let mut ws3 = load::load(&root).expect("chargement 3");
    let rep = presets::apply(&mut ws3, &p);
    assert_eq!(
        (rep.applied, rep.skipped),
        (1, 1),
        "origine modifiée → ignoré"
    );

    // Format v1 (légué) : toujours lu.
    std::fs::write(
        root.join("presets/legacy.preset"),
        "# zoneEdit preset : Legacy\n0:10:Weight = 4\n",
    )
    .expect("écriture v1");
    let (list, _) = presets::load_all(&root, ws.config());
    let legacy = list.iter().find(|p| p.name == "Legacy").expect("v1 chargé");
    assert_eq!(legacy.version, 1);
    assert_eq!(legacy.fields.len(), 1);
    assert_eq!(legacy.fields[0].kind, FieldKind::Weight);
    match &legacy.fields[0].target {
        presets::Target::Line { line, .. } => assert_eq!(*line, 10),
        other => panic!("target v1 attendu, obtenu : {other:?}"),
    }

    let _ = std::fs::remove_dir_all(&root);
}

/// Restaure un fixture (après modification du dossier temporaire).
fn restore_fixture(root: &Path, rel: &str) {
    let src = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join(rel);
    std::fs::copy(&src, root.join(rel)).expect("restauration du fixture");
}

/// Fidélité des fichiers : UTF-8 strict, fins de ligne mixtes préservées
/// (document sans perte), aucun résidu `.tmp` après export.
#[test]
fn fixture_file_fidelity() {
    let (root, out) = copy_fixtures_to_temp("fidelity");
    let rel = "GameData/ItemGeneratorPrototypes.cfg";

    // 1. Octets non-UTF-8 : load échoue avec une erreur claire.
    let p = root.join(rel);
    let mut data = std::fs::read(&p).expect("lecture");
    data.push(0xFF);
    std::fs::write(&p, &data).expect("écriture");
    let err = load::load(&root).expect_err("doit être refusé");
    assert!(
        err.to_string().contains("UTF-8"),
        "erreur attendue, obtenu : {err}"
    );
    restore_fixture(&root, rel);

    // 2. Mixed line endings: load ok, export ok (the lossless document
    // keeps the EOLs as they are; the old "mixed line endings refused"
    // constraint no longer exists).
    let text = std::fs::read_to_string(&p).expect("read");
    let mixed = text.replace("Weight = 1\r\n", "Weight = 1\n");
    assert_ne!(mixed, text, "the fixture must contain CRLF");
    std::fs::write(&p, &mixed).expect("write");
    let mut ws = load::load(&root).expect("mixed load ok");
    let e = find(&ws, "DestructibleStash_Medkit_Newbie", "Medkit");
    let k = ws.own_field(e, FieldKind::Weight).unwrap();
    ws.set_text(k, "5");
    let written = export::export(&ws, &out, "zoneedit").expect("export with mixed EOLs (lossless)");
    assert_eq!(written.len(), 1, "one file exported");
    let exp = std::fs::read_to_string(&written[0].path).unwrap();
    // The patch contains the modified block with the new value.
    assert!(
        exp.contains("struct.begin {bpatch}"),
        "missing {{bpatch}}: {exp}"
    );
    assert!(
        exp.contains("Weight = 5"),
        "modified field in the patch: {exp}"
    );
    restore_fixture(&root, rel);

    // 3. Normal export: no .tmp leftover in the tree.
    let mut ws = load::load(&root).expect("load");
    let e = find(&ws, "DestructibleStash_Medkit_Newbie", "Medkit");
    let k = ws.own_field(e, FieldKind::Weight).unwrap();
    ws.set_text(k, "5");
    export::export(&ws, &out, "zoneedit").expect("export");
    fn list_tmp(dir: &Path, out: &mut Vec<PathBuf>) {
        for e in std::fs::read_dir(dir).unwrap().flatten() {
            let p = e.path();
            if p.is_dir() {
                list_tmp(&p, out);
            } else if p.extension().and_then(|x| x.to_str()) == Some("tmp") {
                out.push(p);
            }
        }
    }
    let mut tmps = Vec::new();
    list_tmp(&out, &mut tmps);
    assert!(tmps.is_empty(), "résidus .tmp : {tmps:?}");

    let _ = std::fs::remove_dir_all(&root);
    let _ = std::fs::remove_dir_all(&out);
}

/// parse_value : formats flottants cfg, valeurs vides, négatifs, exposants.
#[test]
fn edge_cases_parse_value() {
    use zoneedit::domain::value::parse_value;
    assert_eq!(parse_value("0.f"), Some(0.0));
    assert_eq!(parse_value("0.29f"), Some(0.29));
    assert_eq!(parse_value("1.5f"), Some(1.5));
    assert_eq!(parse_value("100.f"), Some(100.0));
    assert_eq!(parse_value("1000f"), Some(1000.0));
    assert_eq!(parse_value("-1f"), Some(-1.0));
    assert_eq!(parse_value("1e3"), Some(1000.0));
    assert_eq!(parse_value("  42  "), Some(42.0));
    assert_eq!(parse_value(""), None);
    assert_eq!(parse_value("   "), None);
    assert_eq!(parse_value("12f34"), None);
    assert_eq!(parse_value("abc"), None);
}

/// FileId : détection d'une modification externe puis rechargement.
#[test]
fn fixture_file_staleness() {
    let (root, _out) = copy_fixtures_to_temp("stale");
    let ws = load::load(&root).expect("chargement");
    assert!(stale_files(&ws).is_empty(), "rien de périmé au chargement");

    // Changement de taille sur disque → périmé.
    let p = root.join("GameData/ItemGeneratorPrototypes.cfg");
    let mut data = std::fs::read(&p).expect("lecture");
    data.push(b'\n');
    std::fs::write(&p, &data).expect("réécriture");
    assert_eq!(
        stale_files(&ws),
        vec!["ItemGeneratorPrototypes".to_string()],
        "fichier modifié détecté"
    );

    // Rechargement : plus rien de périmé.
    let ws2 = load::load(&root).expect("rechargement");
    assert!(stale_files(&ws2).is_empty());

    // Suppression → périmé (fichier illisible).
    let ws3 = load::load(&root).expect("chargement 3");
    std::fs::remove_file(&p).expect("suppression");
    assert_eq!(
        stale_files(&ws3),
        vec!["ItemGeneratorPrototypes".to_string()]
    );

    let _ = std::fs::remove_dir_all(&root);
    let _ = std::fs::remove_dir_all(&_out);
}

#[test]
fn delete_preset_removes_file() {
    let root = std::env::temp_dir().join(format!("ze_del_{}", std::process::id()));
    let _ = std::fs::create_dir_all(root.join("presets"));
    let p = root.join("presets").join("my_preset.preset");
    std::fs::write(&p, "# zoneEdit preset : My preset\n").expect("écriture");
    let cfg = Config::default_arc();
    assert!(presets::delete(&root, &cfg, "My preset").is_ok());
    assert!(!p.exists());
    // Deuxième suppression : plus là.
    assert!(presets::delete(&root, &cfg, "My preset").is_err());
    let _ = std::fs::remove_dir_all(&root);
}

// ---------------------------------------------------------------------------
// Weapon stats (three `WeaponData/*.cfg` files, `multi_category`).
// ---------------------------------------------------------------------------

/// Fixtures : the weapon files parse (CRLF, 3- and 4-space indents, list
/// scopes) and each weapon block belongs to ALL the stat families of its
/// file (the `multi_category` model).
#[test]
fn fixture_weapons_multi_category() {
    let ws = load_fixtures();
    let cfg = ws.config();
    let cats = [
        "WeaponFire",
        "WeaponAiming",
        "WeaponRecoil",
        "WeaponJams",
        "WeaponReload",
        "WeaponHeating",
        "PlayerWeaponDamage",
        "PlayerWeaponDisplay",
        "NpcWeaponCombat",
    ]
    .map(|id| {
        cfg.category_id(id)
            .unwrap_or_else(|| panic!("category {id}"))
    });
    let (fire, aiming, recoil, jams, reload, heating, pld, pld_ui, npc) = (
        cats[0], cats[1], cats[2], cats[3], cats[4], cats[5], cats[6], cats[7], cats[8],
    );

    // GeneralSetup: all three blocks (root template, family template, gun —
    // 3-level inheritance, like the real file) belong to the 6
    // `weapon_general` families.
    for block in ["TemplateWeapon", "TemplateTestPistol", "GunTestPM_HG"] {
        let id = find_id(&ws, block, block);
        for &c in [&fire, &aiming, &recoil, &jams, &reload, &heating] {
            assert!(
                ws.in_category(id, c),
                "{block} must belong to category {c:?}"
            );
        }
    }

    // Flat + transparent-scope fields of the weapon block.
    let gun = find(&ws, "GunTestPM_HG", "GunTestPM_HG");
    assert_eq!(num(val(&ws, gun, FieldKind::FireInterval)), 0.09);
    assert_eq!(num(val(&ws, gun, FieldKind::AimingTime)), 0.25);
    assert_eq!(
        num(val(&ws, gun, FieldKind::RecoilRadius)),
        300.0,
        "RecoilRadius inside the transparent RecoilParams scope"
    );
    assert_eq!(num(val(&ws, gun, FieldKind::FireQueueCount)), 1.0);

    // Player: the two blocks belong to the 2 `weapon_player` families.
    for block in ["TemplatePistol", "GunTestPM_HG_Player"] {
        let id = find_id(&ws, block, block);
        assert!(ws.in_category(id, pld), "{block} in PlayerWeaponDamage");
        assert!(ws.in_category(id, pld_ui), "{block} in PlayerWeaponDisplay");
    }
    let player = find(&ws, "GunTestPM_HG_Player", "GunTestPM_HG_Player");
    assert_eq!(num(val(&ws, player, FieldKind::BaseDamage)), 12.5);
    assert_eq!(
        num(val(&ws, player, FieldKind::ChanceBleedingPerShot)),
        10.0,
        "percent suffix value"
    );

    // NPC (4-space indent): one block in the `weapon_npc` family.
    let npc_id = find_id(&ws, "GunTestPM_HG_NPC", "GunTestPM_HG_NPC");
    assert!(ws.in_category(npc_id, npc));
    let np = ws.entry(npc_id);
    assert_eq!(num(val(&ws, np, FieldKind::BaseBleeding)), 20.0);
    assert_eq!(
        num(val(&ws, np, FieldKind::DispersionRadiusZombieAddend)),
        30.0
    );

    // Weapon entries are not leaked into the item categories (and vice
    // versa): one entry = its file's families only.
    let med = cfg.category_id("MedCrate").unwrap();
    assert!(!ws.in_category(npc_id, med));
}

/// Real data : the three weapon files load, parse byte-exact and expose the
/// expected blocks (92 / 66 / 75 blocks vanilla).
#[test]
fn real_weapons_roundtrip() {
    let ws = load_root();
    for path in [
        "GameData/WeaponData/WeaponGeneralSetupPrototypes.cfg",
        "GameData/WeaponData/CharacterWeaponSettingsPrototypes/PlayerWeaponSettingsPrototypes.cfg",
        "GameData/WeaponData/CharacterWeaponSettingsPrototypes/NPCWeaponSettingsPrototypes.cfg",
    ] {
        let fi = ws
            .files()
            .iter()
            .position(|f| f.spec.path == path)
            .unwrap_or_else(|| panic!("fichier absent : {path}"));
        let rendered = export::render(&ws, fi).expect("rendu du fichier");
        let raw = std::fs::read_to_string(ws.files()[fi].path.clone()).expect("lecture");
        assert_eq!(rendered, raw, "aller-retour non byte-exact : {path}");
    }

    let cfg = ws.config();
    let n = |id: &str| {
        ws.config()
            .category_id(id)
            .unwrap_or_else(|| panic!("catégorie {id}"))
    };
    let fire = n("WeaponFire");
    let pld = n("PlayerWeaponDamage");
    let npc = n("NpcWeaponCombat");
    assert_eq!(
        ws.entry_ids()
            .filter(|&id| ws.in_category(id, fire))
            .count(),
        92,
        "92 blocs WeaponGeneralSetup"
    );
    assert_eq!(
        ws.entry_ids().filter(|&id| ws.in_category(id, pld)).count(),
        66,
        "66 blocs PlayerWeaponSettings"
    );
    assert_eq!(
        ws.entry_ids().filter(|&id| ws.in_category(id, npc)).count(),
        75,
        "75 blocs NPCWeaponSettings"
    );

    // Vanilla values (GunPM_HG / GunPM_HG_Player / GunPM_HG_NPC).
    let gun = find(&ws, "GunPM_HG", "GunPM_HG");
    assert_eq!(num(val(&ws, gun, FieldKind::FireInterval)), 0.09);
    assert_eq!(num(val(&ws, gun, FieldKind::RecoilRadius)), 300.0);
    let player = find(&ws, "GunPM_HG_Player", "GunPM_HG_Player");
    assert_eq!(num(val(&ws, player, FieldKind::BaseDamage)), 30.0);
    assert_eq!(
        num(val(&ws, player, FieldKind::ChanceBleedingPerShot)),
        10.0
    );
    assert_eq!(num(val(&ws, player, FieldKind::AccuracyUI)), 0.4);
    let np = find(&ws, "GunPM_HG_NPC", "GunPM_HG_NPC");
    assert_eq!(num(val(&ws, np, FieldKind::BaseDamage)), 8.0);
    let _ = cfg;
}
