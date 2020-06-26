use crate::Reaper;
use reaper_medium::{Db, ReaperVolumeValue, VolumeSliderValue};
use std::fmt;

pub struct Volume {
    normalized_value: f64,
}

const LN10_OVER_TWENTY: f64 = 0.115_129_254_649_702_28;

impl Volume {
    // TODO Attention! Because of the fact that REAPER allows exceeding the soft maximum of 12 dB,
    //  the VolumeSliderValue can go beyond 1000, which means that this "normalized value" can go
    //  beyond 1.0! Maybe we should call that value range SoftNormalizedValue.
    pub fn from_soft_normalized_value(normalized_value: f64) -> Volume {
        assert!(0.0 <= normalized_value || normalized_value.is_nan());
        Volume { normalized_value }
    }

    pub fn from_reaper_value(reaper_value: ReaperVolumeValue) -> Volume {
        let raw_db = reaper_value.get().ln() / LN10_OVER_TWENTY;
        let db = if raw_db == f64::NEG_INFINITY {
            // REAPER doesn't represent negative infinity as f64::NEG_INFINITY, so we must replace
            // this with REAPER's negative infinity.
            Db::MINUS_INF
        } else {
            Db::new(raw_db)
        };
        Volume::from_db(db)
    }

    pub fn from_db(db: Db) -> Volume {
        Volume::from_soft_normalized_value(
            Reaper::get().medium_reaper().db2slider(db).get() / 1000.0,
        )
    }

    pub fn soft_normalized_value(&self) -> f64 {
        self.normalized_value
    }

    pub fn reaper_value(&self) -> ReaperVolumeValue {
        ReaperVolumeValue::new((self.db().get() * LN10_OVER_TWENTY).exp())
    }

    pub fn db(&self) -> Db {
        Reaper::get()
            .medium_reaper()
            .slider2db(VolumeSliderValue::new(self.normalized_value * 1000.0))
    }
}

impl fmt::Display for Volume {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let vol_string = Reaper::get()
            .medium_reaper()
            .mk_vol_str(self.reaper_value())
            .into_string();
        write!(f, "{}", vol_string)
    }
}
