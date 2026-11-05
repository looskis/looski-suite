// SPDX-License-Identifier: AGPL-3.0-only
//! Similarity is evidence, never a calibrated probability or an identity decision.
use std::collections::BTreeMap;
pub(super) type Signals = BTreeMap<String, String>;
pub(super) const WEIGHTS: &[(&str, f64)] = &[
    ("platform", 2.),
    ("browser", 2.),
    ("major", 0.25),
    ("gpu", 3.),
    ("canvas", 3.),
    ("audio", 2.),
    ("screen", 1.),
    ("timezone", 1.),
    ("languages", 0.5),
    ("hardware", 1.),
];
pub(super) fn score(current: &Signals, old: &Signals) -> Option<f64> {
    for key in ["platform", "browser"] {
        if current.get(key).is_none() || current.get(key) != old.get(key) {
            return None;
        }
    }
    let major = |v: &Signals| v.get("major").and_then(|s| s.parse::<u32>().ok());
    if let (Some(a), Some(b)) = (major(current), major(old))
        && a < b
    {
        return None;
    }
    // Missing signals never count as agreement. Require three substantial signals
    // beyond browser/platform before even suggesting a match.
    let strong = ["gpu", "canvas", "audio", "screen", "hardware"];
    if strong
        .iter()
        .filter(|k| current.contains_key(**k) && old.contains_key(**k))
        .count()
        < 3
    {
        return None;
    }
    let mut same = 0.;
    let mut total = 0.;
    for (key, weight) in WEIGHTS {
        if current.contains_key(*key) || old.contains_key(*key) {
            total += weight;
            if current.get(*key).is_some() && current.get(*key) == old.get(*key) {
                same += weight;
            }
        }
    }
    Some(same / total)
}
#[cfg(test)]
mod tests {
    use super::*;
    fn signals() -> Signals {
        [
            ("platform", "Mac"),
            ("browser", "Chrome"),
            ("major", "140"),
            ("gpu", "gpu"),
            ("canvas", "canvas"),
            ("audio", "audio"),
            ("screen", "screen"),
        ]
        .into_iter()
        .map(|(a, b)| (a.into(), b.into()))
        .collect()
    }
    #[test]
    fn sparse_signals_do_not_link() {
        let s = [
            ("platform".into(), "Mac".into()),
            ("browser".into(), "Safari".into()),
        ]
        .into();
        assert_eq!(score(&s, &s), None);
    }
    #[test]
    fn veto_platform_family_and_downgrade() {
        for (key, value) in [
            ("platform", "Windows"),
            ("browser", "Safari"),
            ("major", "139"),
        ] {
            let mut s = signals();
            s.insert(key.into(), value.into());
            assert_eq!(score(&s, &signals()), None);
        }
    }
    #[test]
    fn drift_is_not_certainty() {
        let mut s = signals();
        assert_eq!(score(&s, &s), Some(1.));
        s.insert("canvas".into(), "randomized".into());
        assert!(score(&s, &signals()).unwrap() < 0.9);
        let mut s = signals();
        s.insert("major".into(), "141".into());
        assert!(score(&s, &signals()).unwrap() > 0.95);
    }
}
