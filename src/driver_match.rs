use crate::driver_index::{normalize_hardware_id, DriverArch, HardwareEntry, InfInfo};
use crate::hardware::HardwareInfo;
use crate::utils::utils::compare_version;
use std::cmp::Ordering;
use std::collections::{BTreeSet, HashMap};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum MatchType {
    HardwareToHardware,
    CompatibleToHardware,
    HardwareToCompatible,
    CompatibleToCompatible,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MatchRank {
    pub signature: u8,
    pub feature_score: u8,
    pub match_type: MatchType,
    pub device_id_position: usize,
    pub inf_id_position: usize,
}

impl Ord for MatchRank {
    fn cmp(&self, other: &Self) -> Ordering {
        self.signature
            .cmp(&other.signature)
            .then_with(|| self.feature_score.cmp(&other.feature_score))
            .then_with(|| self.match_type.cmp(&other.match_type))
            .then_with(|| self.device_id_position.cmp(&other.device_id_position))
            .then_with(|| self.inf_id_position.cmp(&other.inf_id_position))
    }
}

impl PartialOrd for MatchRank {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

#[derive(Debug, Clone)]
pub struct MatchContext {
    pub arch: DriverArch,
    pub os_version: String,
}

#[derive(Debug, Clone)]
pub struct DriverMatch<'a> {
    pub inf: &'a InfInfo,
    pub entry: &'a HardwareEntry,
    pub rank: MatchRank,
}

/// In-memory reverse index from normalized PnP identifiers to INF entries.
pub struct DriverLookup<'a> {
    drivers: &'a [InfInfo],
    by_id: HashMap<String, Vec<(usize, usize)>>,
}

impl<'a> DriverLookup<'a> {
    pub fn new(drivers: &'a [InfInfo]) -> Self {
        let mut by_id: HashMap<String, Vec<(usize, usize)>> = HashMap::new();
        for (inf_index, inf) in drivers.iter().enumerate() {
            for (entry_index, entry) in inf.hardware.iter().enumerate() {
                let ids = std::iter::once(&entry.hardware_id).chain(&entry.compatible_ids);
                for id in ids.filter_map(|id| normalize_hardware_id(id)) {
                    let locations = by_id.entry(id).or_default();
                    if locations.last() != Some(&(inf_index, entry_index)) {
                        locations.push((inf_index, entry_index));
                    }
                }
            }
        }
        Self { drivers, by_id }
    }

    pub fn match_devices<'d>(
        &self,
        devices: &'d [HardwareInfo],
        context: &MatchContext,
        class_filter: Option<&[String]>,
        class_exclude: Option<&[String]>,
    ) -> Vec<(&'d HardwareInfo, Vec<DriverMatch<'a>>)> {
        let explicit_supplemental = class_filter.is_some_and(|classes| {
            classes.iter().any(|class| is_supplemental_class(class))
        });
        let mut results = Vec::new();

        for device in devices {
            let mut locations = BTreeSet::new();
            for id in device
                .hardware_id
                .iter()
                .chain(&device.compatible_id)
                .filter_map(|id| normalize_hardware_id(id))
            {
                if let Some(matches) = self.by_id.get(&id) {
                    locations.extend(matches.iter().copied());
                }
            }

            let mut best_by_inf: HashMap<usize, DriverMatch<'a>> = HashMap::new();
            for (inf_index, entry_index) in locations {
                let inf = &self.drivers[inf_index];
                if !class_allowed(&inf.class, class_filter, class_exclude) {
                    continue;
                }
                let entry = &inf.hardware[entry_index];
                if !entry_allowed(entry, context) {
                    continue;
                }
                let Some(rank) = calculate_rank(device, inf, entry) else {
                    continue;
                };
                let candidate = DriverMatch { inf, entry, rank };
                best_by_inf
                    .entry(inf_index)
                    .and_modify(|current| {
                        if compare_candidates(&candidate, current).is_lt() {
                            *current = candidate.clone();
                        }
                    })
                    .or_insert(candidate);
            }

            let mut candidates: Vec<_> = best_by_inf.into_values().collect();
            if !explicit_supplemental
                && candidates
                    .iter()
                    .any(|candidate| !is_supplemental_class(&candidate.inf.class))
            {
                candidates.retain(|candidate| !is_supplemental_class(&candidate.inf.class));
            }
            candidates.sort_by(compare_candidates);
            if !candidates.is_empty() {
                results.push((device, candidates));
            }
        }
        results
    }
}

pub fn match_drivers<'a>(
    devices: &'a [HardwareInfo],
    drivers: &'a [InfInfo],
    context: &MatchContext,
    class_filter: Option<&[String]>,
    class_exclude: Option<&[String]>,
) -> Vec<(&'a HardwareInfo, Vec<DriverMatch<'a>>)> {
    DriverLookup::new(drivers).match_devices(
        devices,
        context,
        class_filter,
        class_exclude,
    )
}

fn class_allowed(
    class: &str,
    class_filter: Option<&[String]>,
    class_exclude: Option<&[String]>,
) -> bool {
    if class_filter.is_some_and(|classes| {
        !classes
            .iter()
            .any(|candidate| class.eq_ignore_ascii_case(candidate))
    }) {
        return false;
    }
    !class_exclude.is_some_and(|classes| {
        classes
            .iter()
            .any(|candidate| class.eq_ignore_ascii_case(candidate))
    })
}

fn entry_allowed(entry: &HardwareEntry, context: &MatchContext) -> bool {
    (entry.arch == context.arch || entry.arch == DriverArch::Nt)
        && (entry.min_os_version.is_empty()
            || compare_version(&entry.min_os_version, &context.os_version) != Ordering::Greater)
}

fn calculate_rank(
    device: &HardwareInfo,
    inf: &InfInfo,
    entry: &HardwareEntry,
) -> Option<MatchRank> {
    let inf_hardware_id = normalize_hardware_id(&entry.hardware_id)?;
    let inf_compatible_ids: Vec<String> = entry
        .compatible_ids
        .iter()
        .filter_map(|id| normalize_hardware_id(id))
        .collect();

    let combinations = [
        (&device.hardware_id, MatchType::HardwareToHardware, true),
        (&device.compatible_id, MatchType::CompatibleToHardware, true),
        (&device.hardware_id, MatchType::HardwareToCompatible, false),
        (
            &device.compatible_id,
            MatchType::CompatibleToCompatible,
            false,
        ),
    ];
    for (device_ids, match_type, match_hardware) in combinations {
        for (device_position, device_id) in device_ids.iter().enumerate() {
            let Some(device_id) = normalize_hardware_id(device_id) else {
                continue;
            };
            let inf_position = if match_hardware {
                (device_id == inf_hardware_id).then_some(0)
            } else {
                inf_compatible_ids.iter().position(|id| id == &device_id)
            };
            if let Some(inf_id_position) = inf_position {
                return Some(MatchRank {
                    signature: inf.signature,
                    feature_score: entry.feature_score,
                    match_type,
                    device_id_position: device_position,
                    inf_id_position,
                });
            }
        }
    }
    None
}

fn compare_candidates(a: &DriverMatch<'_>, b: &DriverMatch<'_>) -> Ordering {
    a.rank
        .cmp(&b.rank)
        .then_with(|| b.inf.date.cmp(&a.inf.date))
        .then_with(|| compare_version(&b.inf.version, &a.inf.version))
        .then_with(|| {
            a.inf
                .path
                .to_ascii_lowercase()
                .cmp(&b.inf.path.to_ascii_lowercase())
        })
}

fn is_supplemental_class(class: &str) -> bool {
    class.eq_ignore_ascii_case("Extension")
        || class.eq_ignore_ascii_case("SoftwareComponent")
}
