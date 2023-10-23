use crate::{CachableTextSize, TextRangeWrapper};

use rustpython_parser::text_size::TextSize;
use serde::{de, Deserialize, Deserializer, Serialize, Serializer};

impl Serialize for CachableTextSize {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.raw.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for CachableTextSize {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        u32::deserialize(deserializer).map(CachableTextSize::from)
    }
}

impl Serialize for TextRangeWrapper {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        (self.0.start().to_u32(), self.0.end().to_u32()).serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for TextRangeWrapper {
    #[allow(clippy::nonminimal_bool)]
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let (start, end) = Deserialize::deserialize(deserializer)?;
        if !(start <= end) {
            return Err(de::Error::custom(format!(
                "invalid range: {start:?}..{end:?}"
            )));
        }
        Ok(TextRangeWrapper::new(
            TextSize::new(start),
            TextSize::new(end),
        ))
    }
}
