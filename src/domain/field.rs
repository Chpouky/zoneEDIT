//! Editable fields: field type, identifier, current value.

use serde::{Deserialize, Serialize};

use super::value::parse_value;

/// Type of an editable field. The variant name is the name of the key
/// in the cfg files; labels, bounds and unit come from the
/// configuration ([`crate::config::FieldSpec`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum FieldKind {
    /// Relative weight of the entry in its generator.
    Weight,
    /// Spawn chance (0..1, `f` suffix).
    Chance,
    /// Minimum number of objects.
    MinCount,
    /// Maximum number of objects.
    MaxCount,
    /// Ammunition: minimum.
    AmmoMinCount,
    /// Ammunition: maximum.
    AmmoMaxCount,
    /// Weapon durability: minimum (fraction of the max durability, 0..1).
    MinDurability,
    /// Weapon durability: maximum (fraction of the max durability, 0..1).
    MaxDurability,
    /// Ammunition of the main weapon.
    MainWeaponAmmoCount,
    /// Minimum spawn chance (stash) (0..1, `f` suffix).
    MinSpawnChance,
    /// Maximum spawn chance (stash) (0..1, `f` suffix).
    MaxSpawnChance,
    /// Anomaly: radius of the interaction volume (cm).
    Radius,
    /// Anomaly: seconds between two triggers.
    Cooldown,
    /// Anomaly: duration of the effect (seconds).
    Duration,
    /// Anomaly: delay before the effect kicks in (seconds).
    ActivationDelay,
    /// Anomaly: required movement speed to activate (m/s).
    ActivationSpeedThreshold,
    /// Anomaly: vertical offset of the anomaly (cm).
    DefaultHeightOffset,
    /// Anomaly: height of the collision volume (cm).
    CollisionHeight,
    /// Anomaly: radius within which the anomaly prioritises its movement target.
    MovementLocationPriorityRadius,
    /// Anomaly: movement speed.
    MovementSpeed,
    /// Anomaly: minimum number of fake artifacts it can spawn.
    FakeArtifactSpawnAmountMin,
    /// Anomaly: maximum number of fake artifacts it can spawn.
    FakeArtifactSpawnAmountMax,
    /// World artifact spawner: maximum number of artifacts in the zone.
    ArtifactCount,
    /// World artifact spawner: seconds between two spawns (minimum).
    MinCooldown,
    /// World artifact spawner: seconds between two spawns (maximum).
    MaxCooldown,
    /// World artifact spawner: base spawn chance (%).
    SpawnChanceBase,
    /// World artifact spawner: bonus spawn chance (%).
    SpawnChanceBonus,
    /// World artifact spawner: chance of the "common" rarity tier (%).
    RarityChanceCommon,
    /// World artifact spawner: chance of the "uncommon" rarity tier (%).
    RarityChanceUncommon,
    /// World artifact spawner: chance of the "rare" rarity tier (%).
    RarityChanceRare,
    /// World artifact spawner: chance of the "epic" rarity tier (%).
    RarityChanceEpic,
}

impl FieldKind {
    /// All the types, in declaration order.
    pub const ALL: [FieldKind; 31] = [
        Self::Weight,
        Self::Chance,
        Self::MinCount,
        Self::MaxCount,
        Self::AmmoMinCount,
        Self::AmmoMaxCount,
        Self::MinDurability,
        Self::MaxDurability,
        Self::MainWeaponAmmoCount,
        Self::MinSpawnChance,
        Self::MaxSpawnChance,
        Self::Radius,
        Self::Cooldown,
        Self::Duration,
        Self::ActivationDelay,
        Self::ActivationSpeedThreshold,
        Self::DefaultHeightOffset,
        Self::CollisionHeight,
        Self::MovementLocationPriorityRadius,
        Self::MovementSpeed,
        Self::FakeArtifactSpawnAmountMin,
        Self::FakeArtifactSpawnAmountMax,
        Self::ArtifactCount,
        Self::MinCooldown,
        Self::MaxCooldown,
        Self::SpawnChanceBase,
        Self::SpawnChanceBonus,
        Self::RarityChanceCommon,
        Self::RarityChanceUncommon,
        Self::RarityChanceRare,
        Self::RarityChanceEpic,
    ];
    /// Number of field types.
    pub const COUNT: usize = Self::ALL.len();

    /// Stable index (0..COUNT) for the tables indexed by type.
    pub fn index(self) -> usize {
        self as usize
    }

    /// Name of the key as it appears in the cfgs.
    pub fn key(self) -> &'static str {
        match self {
            Self::Weight => "Weight",
            Self::Chance => "Chance",
            Self::MinCount => "MinCount",
            Self::MaxCount => "MaxCount",
            Self::AmmoMinCount => "AmmoMinCount",
            Self::AmmoMaxCount => "AmmoMaxCount",
            Self::MinDurability => "MinDurability",
            Self::MaxDurability => "MaxDurability",
            Self::MainWeaponAmmoCount => "MainWeaponAmmoCount",
            Self::MinSpawnChance => "MinSpawnChance",
            Self::MaxSpawnChance => "MaxSpawnChance",
            Self::Radius => "Radius",
            Self::Cooldown => "Cooldown",
            Self::Duration => "Duration",
            Self::ActivationDelay => "ActivationDelay",
            Self::ActivationSpeedThreshold => "ActivationSpeedThreshold",
            Self::DefaultHeightOffset => "DefaultHeightOffset",
            Self::CollisionHeight => "CollisionHeight",
            Self::MovementLocationPriorityRadius => "MovementLocationPriorityRadius",
            Self::MovementSpeed => "MovementSpeed",
            Self::FakeArtifactSpawnAmountMin => "FakeArtifactSpawnAmountMin",
            Self::FakeArtifactSpawnAmountMax => "FakeArtifactSpawnAmountMax",
            Self::ArtifactCount => "ArtifactCount",
            Self::MinCooldown => "MinCooldown",
            Self::MaxCooldown => "MaxCooldown",
            Self::SpawnChanceBase => "SpawnChanceBase",
            Self::SpawnChanceBonus => "SpawnChanceBonus",
            Self::RarityChanceCommon => "RarityChanceCommon",
            Self::RarityChanceUncommon => "RarityChanceUncommon",
            Self::RarityChanceRare => "RarityChanceRare",
            Self::RarityChanceEpic => "RarityChanceEpic",
        }
    }

    /// Inverse of [`FieldKind::key`].
    pub fn from_key(key: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|k| k.key() == key)
    }
}

/// Identifier of a field in the workspace (index in the arena).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct FieldId(pub u32);

impl FieldId {
    /// Index in the arena.
    pub fn idx(self) -> usize {
        self.0 as usize
    }
}

/// Location of a field in the files: stable lookup key
/// (presets by line number, tests).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct FieldLoc {
    /// Index du fichier dans le workspace.
    pub file: usize,
    /// Ligne 1-based.
    pub line: usize,
    /// Type du champ.
    pub kind: FieldKind,
}

/// An editable field. A single instance per (file, line): the group
/// fields (caliber / set) are shared by all the entries of the group.
///
/// The value is only modifiable via the [`super::Workspace`] (counters,
/// revision and undo history kept up to date).
#[derive(Debug, Clone)]
pub struct Field {
    /// Type du champ.
    pub kind: FieldKind,
    /// The key as written in the file (aliases included: `weight`,
    /// `Count`, …) — the patch must re-emit it, not the canonical key.
    key: Box<str>,
    /// Index du fichier dans le workspace.
    pub file: usize,
    /// Ligne 1-based.
    pub line: usize,
    original: Box<str>,
    current: Option<Box<str>>,
    /// Numeric value of the effective value (`None` = non-numeric).
    num: Option<f64>,
    /// Effective value compliant with the schema of the field.
    valid: bool,
}

impl Field {
    pub(crate) fn new(
        kind: FieldKind,
        file: usize,
        line: usize,
        key: &str,
        original: &str,
    ) -> Self {
        Self {
            kind,
            key: key.to_string().into(),
            file,
            line,
            original: original.into(),
            current: None,
            num: parse_value(original),
            valid: true,
        }
    }

    /// The key as written in the file (e.g. `weight`, `Count`).
    pub fn key(&self) -> &str {
        &self.key
    }

    /// Emplacement stable dans les fichiers.
    pub fn loc(&self) -> FieldLoc {
        FieldLoc {
            file: self.file,
            line: self.line,
            kind: self.kind,
        }
    }

    /// Value as written in the original file.
    pub fn original(&self) -> &str {
        &self.original
    }

    /// Modified value (`None` = identical to the original).
    pub fn current(&self) -> Option<&str> {
        self.current.as_deref()
    }

    /// Effective value (modified or original).
    pub fn value(&self) -> &str {
        self.current.as_deref().unwrap_or(&self.original)
    }

    /// Valeur effective en nombre.
    pub fn num(&self) -> Option<f64> {
        self.num
    }

    /// The value has been modified.
    pub fn is_dirty(&self) -> bool {
        self.current.is_some()
    }

    /// `false` if the effective value is modified AND invalid (the game's
    /// original values are not checked).
    pub fn is_valid(&self) -> bool {
        !self.is_dirty() || self.valid
    }

    /// Replaces the current value (normalized: identical to the original →
    /// `None`) and recomputes the cache. Reserved to the workspace.
    pub(crate) fn set_current(&mut self, current: Option<Box<str>>, valid: bool) {
        self.current = current.filter(|c| **c != *self.original);
        self.num = parse_value(self.value());
        self.valid = valid;
    }
}
