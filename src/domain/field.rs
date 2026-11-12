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
    // --- Weapons (flat stats of the three `WeaponData/*.cfg` files) ---
    /// Weapon: seconds between two shots (fire interval).
    FireInterval,
    /// Weapon: seconds between the recoil impulses (recoil interval).
    RecoilInterval,
    /// Weapon: number of shots grouped in one burst (fire queue).
    FireQueueCount,
    /// Weapon: seconds of cooldown after the last shot of a queue.
    CooldownTime,
    /// Weapon: seconds to unload (swap out) the weapon.
    UnloadTime,
    /// Weapon: offset (cm) where projectiles spawn (launchers).
    ProjectileSpawnOffset,
    /// Weapon: seconds to reach full aim (ADS time).
    AimingTime,
    /// Weapon: seconds to reach the offset (stand) aiming pose.
    OffsetAimingTime,
    /// Weapon: seconds to lean while aiming.
    LeanAimingTime,
    /// Weapon: seconds to restore the lean after aiming.
    LeanAimingRestoreTime,
    /// Weapon: FOV multiplier while aiming (1 = unchanged).
    AimingFOVModifier,
    /// Weapon: FOV multiplier in the offset aiming pose.
    OffsetAimingFOVModifier,
    /// Weapon: movement speed multiplier while aiming.
    AimingMovementSpeedModifier,
    /// Weapon: hold-breath stamina drain multiplier while aiming.
    HoldBreathDrainModifier,
    /// Weapon: radius (cm) of the recoil pattern.
    RecoilRadius,
    /// Weapon: interval of the recoil pattern (texture keys).
    RecoilPatternInterval,
    /// Weapon: maximum recoil pitch (degrees).
    RecoilMaxPitch,
    /// Weapon: shot index where the recoil radius starts normalizing.
    LinearCurveStartShot,
    /// Weapon: shots over which the recoil radius normalizes.
    LinearCurveShotsToNormalize,
    /// Weapon: start key index of the camera recoil pattern (aim).
    CameraAimRecoilPatternLoopStartKeyIndex,
    /// Weapon: start key index of the camera recoil pattern (hip fire).
    CameraHipRecoilPatternLoopStartKeyIndex,
    /// Weapon: dispersion radius speed coefficient.
    SpeedCoef,
    /// Weapon: radius (cm) of the dispersion on the first shot.
    FirstShotDispersionRadius,
    /// Weapon: minimum jam chance (0..1).
    MinJamChance,
    /// Weapon: maximum jam chance.
    MaxJamChance,
    /// Weapon: weapon durability (0..1) below which jams are minimum.
    MinJamDurabilityThreshold,
    /// Weapon: weapon durability (0..1) above which jams are maximum.
    MaxJamDurabilityThreshold,
    /// Weapon: maximum number of bullets in the magazine.
    MaxAmmo,
    /// Weapon: number of bullets consumed per shot.
    AmmoPerShot,
    /// Weapon: bullets reloaded per bullet of the old magazine.
    PerBulletReloadingAmmoCount,
    /// Weapon: extra bullets added after a reload.
    AdditionalBulletsAfterReloadingCount,
    /// Weapon: reload time multiplier for a single-bullet reload.
    SingleBulletReloadTimeMultiplier,
    /// Weapon: loaded ammo of a dead NPC (minimum, -1 = none).
    MinDeadNPCLoadedAmmoCount,
    /// Weapon: loaded ammo of a dead NPC (maximum, -1 = none).
    MaxDeadNPCLoadedAmmoCount,
    /// Weapon: maximum heating value of the barrel.
    MaxHeatingValue,
    /// Weapon: heating value added per bullet.
    PerBulletHeatingValue,
    /// Weapon: bullets left in the last clip before the delay.
    LastClipBulletsCount,
    /// Weapon: total bullets left before the last-clip delay.
    LastTotalBulletsCount,
    /// Weapon: damage multiplier of the last-clip bullets.
    LastClipBulletsDamageModifier,
    /// Weapon: damage modifier of the last total bullets.
    LastTotalBulletsDamageModifier,
    /// Weapon settings: base damage per bullet (player / NPC).
    BaseDamage,
    /// Weapon settings: damage dealt to armor (player / NPC).
    ArmorDamage,
    /// Weapon settings: armor penetration (player / NPC).
    ArmorPiercing,
    /// Weapon settings: cover penetration (player).
    CoverPiercing,
    /// Weapon settings: base bleeding chance (0..1 fraction).
    BaseBleeding,
    /// Weapon settings: bleeding chance per shot (percent value, `10%`).
    ChanceBleedingPerShot,
    /// Weapon settings: durability damage per shot (player / NPC).
    DurabilityDamagePerShot,
    /// Weapon settings: minimum effective fire distance (cm).
    EffectiveFireDistanceMin,
    /// Weapon settings: maximum effective fire distance (cm).
    EffectiveFireDistanceMax,
    /// Weapon settings: distance over which the damage drops off (cm).
    FireDistanceDropOff,
    /// Weapon settings: damage modifier at close range.
    MinBulletDistanceDamageModifier,
    /// Weapon settings: armor piercing modifier at close range.
    MinBulletDistanceArmorPiercingModifier,
    /// Weapon settings: length of the damage drop-off zone (cm).
    DistanceDropOffLength,
    /// Weapon settings: vertical bullet drop height (cm).
    BulletDropHeight,
    /// Weapon settings: length of the bullet drop (cm).
    BulletDropLength,
    /// Weapon settings: dispersion radius (player / NPC).
    DispersionRadius,
    /// Weapon settings: NPC dispersion radius addend for zombies.
    DispersionRadiusZombieAddend,
    /// Weapon settings: player accuracy (0..1).
    Accuracy,
    /// Weapon settings: firing loudness (player, 0..1).
    FireLoudness,
    /// Weapon settings: displayed accuracy bar (0..1).
    AccuracyUI,
    /// Weapon settings: displayed rate of fire bar (0..1).
    RateOfFireUI,
    /// Weapon settings: displayed handling bar (0..1).
    HandlingUI,
    /// Weapon settings: displayed damage bar (0..1).
    DamageUI,
    /// Weapon settings: displayed range bar (0..1).
    RangeUI,
    /// Weapon settings: displayed base comfort bar (0..1).
    BaseComfort,
}

impl FieldKind {
    /// All the types, in declaration order.
    pub const ALL: [FieldKind; 96] = [
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
        Self::FireInterval,
        Self::RecoilInterval,
        Self::FireQueueCount,
        Self::CooldownTime,
        Self::UnloadTime,
        Self::ProjectileSpawnOffset,
        Self::AimingTime,
        Self::OffsetAimingTime,
        Self::LeanAimingTime,
        Self::LeanAimingRestoreTime,
        Self::AimingFOVModifier,
        Self::OffsetAimingFOVModifier,
        Self::AimingMovementSpeedModifier,
        Self::HoldBreathDrainModifier,
        Self::RecoilRadius,
        Self::RecoilPatternInterval,
        Self::RecoilMaxPitch,
        Self::LinearCurveStartShot,
        Self::LinearCurveShotsToNormalize,
        Self::CameraAimRecoilPatternLoopStartKeyIndex,
        Self::CameraHipRecoilPatternLoopStartKeyIndex,
        Self::SpeedCoef,
        Self::FirstShotDispersionRadius,
        Self::MinJamChance,
        Self::MaxJamChance,
        Self::MinJamDurabilityThreshold,
        Self::MaxJamDurabilityThreshold,
        Self::MaxAmmo,
        Self::AmmoPerShot,
        Self::PerBulletReloadingAmmoCount,
        Self::AdditionalBulletsAfterReloadingCount,
        Self::SingleBulletReloadTimeMultiplier,
        Self::MinDeadNPCLoadedAmmoCount,
        Self::MaxDeadNPCLoadedAmmoCount,
        Self::MaxHeatingValue,
        Self::PerBulletHeatingValue,
        Self::LastClipBulletsCount,
        Self::LastTotalBulletsCount,
        Self::LastClipBulletsDamageModifier,
        Self::LastTotalBulletsDamageModifier,
        Self::BaseDamage,
        Self::ArmorDamage,
        Self::ArmorPiercing,
        Self::CoverPiercing,
        Self::BaseBleeding,
        Self::ChanceBleedingPerShot,
        Self::DurabilityDamagePerShot,
        Self::EffectiveFireDistanceMin,
        Self::EffectiveFireDistanceMax,
        Self::FireDistanceDropOff,
        Self::MinBulletDistanceDamageModifier,
        Self::MinBulletDistanceArmorPiercingModifier,
        Self::DistanceDropOffLength,
        Self::BulletDropHeight,
        Self::BulletDropLength,
        Self::DispersionRadius,
        Self::DispersionRadiusZombieAddend,
        Self::Accuracy,
        Self::FireLoudness,
        Self::AccuracyUI,
        Self::RateOfFireUI,
        Self::HandlingUI,
        Self::DamageUI,
        Self::RangeUI,
        Self::BaseComfort,
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
            Self::FireInterval => "FireInterval",
            Self::RecoilInterval => "RecoilInterval",
            Self::FireQueueCount => "FireQueueCount",
            Self::CooldownTime => "CooldownTime",
            Self::UnloadTime => "UnloadTime",
            Self::ProjectileSpawnOffset => "ProjectileSpawnOffset",
            Self::AimingTime => "AimingTime",
            Self::OffsetAimingTime => "OffsetAimingTime",
            Self::LeanAimingTime => "LeanAimingTime",
            Self::LeanAimingRestoreTime => "LeanAimingRestoreTime",
            Self::AimingFOVModifier => "AimingFOVModifier",
            Self::OffsetAimingFOVModifier => "OffsetAimingFOVModifier",
            Self::AimingMovementSpeedModifier => "AimingMovementSpeedModifier",
            Self::HoldBreathDrainModifier => "HoldBreathDrainModifier",
            Self::RecoilRadius => "RecoilRadius",
            Self::RecoilPatternInterval => "RecoilPatternInterval",
            Self::RecoilMaxPitch => "RecoilMaxPitch",
            Self::LinearCurveStartShot => "LinearCurveStartShot",
            Self::LinearCurveShotsToNormalize => "LinearCurveShotsToNormalize",
            Self::CameraAimRecoilPatternLoopStartKeyIndex => {
                "CameraAimRecoilPatternLoopStartKeyIndex"
            }
            Self::CameraHipRecoilPatternLoopStartKeyIndex => {
                "CameraHipRecoilPatternLoopStartKeyIndex"
            }
            Self::SpeedCoef => "SpeedCoef",
            Self::FirstShotDispersionRadius => "FirstShotDispersionRadius",
            Self::MinJamChance => "MinJamChance",
            Self::MaxJamChance => "MaxJamChance",
            Self::MinJamDurabilityThreshold => "MinJamDurabilityThreshold",
            Self::MaxJamDurabilityThreshold => "MaxJamDurabilityThreshold",
            Self::MaxAmmo => "MaxAmmo",
            Self::AmmoPerShot => "AmmoPerShot",
            Self::PerBulletReloadingAmmoCount => "PerBulletReloadingAmmoCount",
            Self::AdditionalBulletsAfterReloadingCount => "AdditionalBulletsAfterReloadingCount",
            Self::SingleBulletReloadTimeMultiplier => "SingleBulletReloadTimeMultiplier",
            Self::MinDeadNPCLoadedAmmoCount => "MinDeadNPCLoadedAmmoCount",
            Self::MaxDeadNPCLoadedAmmoCount => "MaxDeadNPCLoadedAmmoCount",
            Self::MaxHeatingValue => "MaxHeatingValue",
            Self::PerBulletHeatingValue => "PerBulletHeatingValue",
            Self::LastClipBulletsCount => "LastClipBulletsCount",
            Self::LastTotalBulletsCount => "LastTotalBulletsCount",
            Self::LastClipBulletsDamageModifier => "LastClipBulletsDamageModifier",
            Self::LastTotalBulletsDamageModifier => "LastTotalBulletsDamageModifier",
            Self::BaseDamage => "BaseDamage",
            Self::ArmorDamage => "ArmorDamage",
            Self::ArmorPiercing => "ArmorPiercing",
            Self::CoverPiercing => "CoverPiercing",
            Self::BaseBleeding => "BaseBleeding",
            Self::ChanceBleedingPerShot => "ChanceBleedingPerShot",
            Self::DurabilityDamagePerShot => "DurabilityDamagePerShot",
            Self::EffectiveFireDistanceMin => "EffectiveFireDistanceMin",
            Self::EffectiveFireDistanceMax => "EffectiveFireDistanceMax",
            Self::FireDistanceDropOff => "FireDistanceDropOff",
            Self::MinBulletDistanceDamageModifier => "MinBulletDistanceDamageModifier",
            Self::MinBulletDistanceArmorPiercingModifier => {
                "MinBulletDistanceArmorPiercingModifier"
            }
            Self::DistanceDropOffLength => "DistanceDropOffLength",
            Self::BulletDropHeight => "BulletDropHeight",
            Self::BulletDropLength => "BulletDropLength",
            Self::DispersionRadius => "DispersionRadius",
            Self::DispersionRadiusZombieAddend => "DispersionRadiusZombieAddend",
            Self::Accuracy => "Accuracy",
            Self::FireLoudness => "FireLoudness",
            Self::AccuracyUI => "AccuracyUI",
            Self::RateOfFireUI => "RateOfFireUI",
            Self::HandlingUI => "HandlingUI",
            Self::DamageUI => "DamageUI",
            Self::RangeUI => "RangeUI",
            Self::BaseComfort => "BaseComfort",
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
