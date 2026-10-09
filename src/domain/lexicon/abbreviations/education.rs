//! Examinations and university initialisms.
use super::{Definition, initialism};
use crate::morphology::Harmony::{FrontFlat, FrontRound};

pub(super) const ENTRIES: &[(&str, Definition)] = &[
    ("YKS", initialism("ye ke se", "se", FrontFlat)),
    ("KPSS", initialism("ke pe se se", "se", FrontFlat)),
    ("LGS", initialism("le ge se", "se", FrontFlat)),
    ("ÖSYM", initialism("ö se ye me", "me", FrontFlat)),
    ("YDS", initialism("ye de se", "se", FrontFlat)),
    ("İTÜ", initialism("i te ü", "ü", FrontRound)),
    ("KTÜ", initialism("ke te ü", "ü", FrontRound)),
    ("YTÜ", initialism("ye te ü", "ü", FrontRound)),
];
