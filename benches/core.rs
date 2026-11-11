//! Benchmarks de référence (criterion) — voir section 5 de `PLANNING.md`.
//!
//! Les fixtures de `tests/fixtures/` sont petites ; on les gonfle par
//! duplication (≈ 870 copies du fichier principal, soit l'ordre de grandeur
//! des données réelles : ~82 k lignes / ~8 k entrées) dans une racine
//! temporaire, pour rester hermétique.

use std::path::Path;

use criterion::{criterion_group, criterion_main, Criterion, Throughput};
use zoneedit::domain::{FieldId, FieldKind, Workspace};
use zoneedit::io::{export, load, presets};

/// Copies du fixture principal (9 entrées / copy → ≈ 7,8 k entrées).
const COPIES: usize = 870;

fn fixture_dir() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
}

/// Builds a hermetic root: the 5 configured files, the main one
/// duplicated `COPIES` times.
fn build_large_root() -> tempfile::TempDir {
    let f = fixture_dir();
    let root = tempfile::tempdir().expect("tempdir");
    let gd = root.path().join("GameData");
    std::fs::create_dir_all(gd.join("ItemGeneratorPrototypes")).expect("mkdir");

    let igp = std::fs::read(f.join("GameData/ItemGeneratorPrototypes.cfg")).expect("fixture");
    let mut big = Vec::with_capacity(igp.len() * COPIES);
    for _ in 0..COPIES {
        big.extend_from_slice(&igp);
    }
    std::fs::write(gd.join("ItemGeneratorPrototypes.cfg"), big).expect("write");

    for rel in [
        "GameData/ItemGeneratorPrototypes/DynamicItemGenerator.cfg",
        "GameData/ItemGeneratorPrototypes/QuestItemGeneratorPrototypes.cfg",
        "GameData/StashPrototypes.cfg",
        "GameData/AnomalyPrototypes.cfg",
        "GameData/ArtifactSpawnerPrototypes.cfg",
    ] {
        std::fs::copy(f.join(rel), root.path().join(rel)).expect("copy fixture");
    }
    root
}

/// Workspace avec ~500 champs `Weight` dirty (pour le benchmark de preset).
fn workspace_with_dirty(root: &Path) -> Workspace {
    let mut ws = load::load(root).expect("load");
    // Les ids sont collectés d'abord : le borrow immuable de `entries()` ne
    // doit pas survivre aux `set_text`.
    let ids: Vec<(FieldId, String)> = ws
        .entries()
        .iter()
        .filter_map(|e| {
            ws.own_field(e, FieldKind::Weight)
                .map(|k| (k, ws.field(k).value().to_string()))
        })
        .take(500)
        .collect();
    for (k, v) in &ids {
        // Valeur forcément différente de l'originale, toujours valide.
        ws.set_text(*k, &format!("{v}0"));
    }
    ws
}

fn bench(c: &mut Criterion) {
    let tmp = build_large_root();
    let root = tmp.path();
    let ws = load::load(root).expect("load");
    let n_entries = ws.entries().len();
    eprintln!(
        "inflated fixture: {n_entries} entries, {} fields, {} lines",
        ws.fields().len(),
        ws.files()[0].doc.line_count()
    );

    // ---- load : découverte + parse parallèle + index -----------------------
    let mut g = c.benchmark_group("load");
    g.throughput(Throughput::Elements(n_entries as u64));
    g.bench_function("load_root", |b| b.iter(|| load::load(root).unwrap()));
    g.finish();

    // ---- render : rendu byte-exact du plus gros fichier -------------------
    let n_bytes = std::fs::metadata(&ws.files()[0].path)
        .expect("metadata")
        .len();
    let mut g = c.benchmark_group("render");
    g.throughput(Throughput::Bytes(n_bytes));
    g.bench_function("render_file0", |b| {
        b.iter(|| export::render(&ws, 0).unwrap());
    });
    g.finish();

    // ---- validation : scan des champs dirty -------------------------------
    let mut g = c.benchmark_group("validation");
    g.throughput(Throughput::Elements(n_entries as u64));
    g.bench_function("validation_errors", |b| b.iter(|| ws.validation_errors()));
    g.finish();

    // ---- save_preset : ~500 champs -----------------------------------------
    let ws2 = workspace_with_dirty(root);
    let n_dirty = ws2.dirty_count();
    let path = presets::save(&ws2, "bench").expect("save");
    let mut g = c.benchmark_group("save_preset");
    g.throughput(Throughput::Elements(n_dirty as u64));
    g.bench_function("save_500_fields", |b| {
        b.iter(|| {
            std::fs::remove_file(&path).ok();
            presets::save(&ws2, "bench").unwrap();
        });
    });
    g.finish();
    std::fs::remove_file(&path).ok();
}

criterion_group!(benches, bench);
criterion_main!(benches);
