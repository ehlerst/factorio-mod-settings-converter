use anyhow::{anyhow, Result};
use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::io::{Read, Write};

#[cfg(target_arch = "wasm32")]
pub mod wasm;

#[derive(Debug, Clone, PartialEq)]
pub struct ImmutableString(pub Option<Vec<u8>>);

impl ImmutableString {
    pub fn load<R: Read>(reader: &mut R) -> Result<Self> {
        let is_none = reader.read_u8()? != 0;
        if is_none {
            Ok(Self(None))
        } else {
            let mut len = reader.read_u8()? as usize;
            if len == 0xff {
                len = reader.read_u32::<LittleEndian>()? as usize;
            }
            let mut buf = vec![0u8; len];
            reader.read_exact(&mut buf)?;
            Ok(Self(Some(buf)))
        }
    }

    pub fn save<W: Write>(&self, writer: &mut W) -> Result<()> {
        match &self.0 {
            None => {
                writer.write_u8(1)?;
            }
            Some(bytes) => {
                writer.write_u8(0)?;
                if bytes.len() >= 0xff {
                    writer.write_u8(0xff)?;
                    writer.write_u32::<LittleEndian>(bytes.len() as u32)?;
                } else {
                    writer.write_u8(bytes.len() as u8)?;
                }
                writer.write_all(bytes)?;
            }
        }
        Ok(())
    }

    pub fn from_string(s: String) -> Self {
        Self(Some(s.into_bytes()))
    }

    pub fn to_string_lossy(&self) -> Option<String> {
        self.0.as_ref().map(|b| String::from_utf8_lossy(b).into_owned())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum PropertyTreeType {
    Null = 0,
    Bool = 1,
    Number = 2,
    String = 3,
    List = 4,
    Dictionary = 5,
    SignedInteger = 6,
    UnsignedInteger = 7,
}

impl PropertyTreeType {
    fn from_u8(v: u8) -> Result<Self> {
        match v {
            0 => Ok(Self::Null),
            1 => Ok(Self::Bool),
            2 => Ok(Self::Number),
            3 => Ok(Self::String),
            4 => Ok(Self::List),
            5 => Ok(Self::Dictionary),
            6 => Ok(Self::SignedInteger),
            7 => Ok(Self::UnsignedInteger),
            _ => Err(anyhow!("Unknown PropertyTree type: {}", v)),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum PropertyTreeValue {
    Null,
    Bool(bool),
    Number(f64),
    String(ImmutableString),
    List(Vec<PropertyTree>),
    Dictionary(Vec<(ImmutableString, PropertyTree)>),
    SignedInteger(i64),
    UnsignedInteger(u64),
}

#[derive(Debug, Clone, PartialEq)]
pub struct PropertyTree {
    pub value: PropertyTreeValue,
    pub any_type: bool,
}

impl PropertyTree {
    pub fn load<R: Read>(reader: &mut R) -> Result<Self> {
        let type_raw = reader.read_u8()?;
        let any_type = reader.read_u8()? != 0;
        let value_type = PropertyTreeType::from_u8(type_raw)?;

        let value = match value_type {
            PropertyTreeType::Null => PropertyTreeValue::Null,
            PropertyTreeType::Bool => {
                let v = reader.read_u8()? != 0;
                PropertyTreeValue::Bool(v)
            }
            PropertyTreeType::Number => {
                let v = reader.read_f64::<LittleEndian>()?;
                PropertyTreeValue::Number(v)
            }
            PropertyTreeType::String => {
                let v = ImmutableString::load(reader)?;
                PropertyTreeValue::String(v)
            }
            PropertyTreeType::List | PropertyTreeType::Dictionary => {
                let count = reader.read_u32::<LittleEndian>()? as usize;
                let mut items = Vec::with_capacity(count);
                for _ in 0..count {
                    let key = ImmutableString::load(reader)?;
                    let item = Self::load(reader)?;
                    items.push((key, item));
                }
                if value_type == PropertyTreeType::List {
                    PropertyTreeValue::List(items.into_iter().map(|(_, v)| v).collect())
                } else {
                    PropertyTreeValue::Dictionary(items)
                }
            }
            PropertyTreeType::SignedInteger => {
                let v = reader.read_i64::<LittleEndian>()?;
                PropertyTreeValue::SignedInteger(v)
            }
            PropertyTreeType::UnsignedInteger => {
                let v = reader.read_u64::<LittleEndian>()?;
                PropertyTreeValue::UnsignedInteger(v)
            }
        };

        Ok(Self { value, any_type })
    }

    pub fn save<W: Write>(&self, writer: &mut W) -> Result<()> {
        let type_val = match &self.value {
            PropertyTreeValue::Null => PropertyTreeType::Null,
            PropertyTreeValue::Bool(_) => PropertyTreeType::Bool,
            PropertyTreeValue::Number(_) => PropertyTreeType::Number,
            PropertyTreeValue::String(_) => PropertyTreeType::String,
            PropertyTreeValue::List(_) => PropertyTreeType::List,
            PropertyTreeValue::Dictionary(_) => PropertyTreeType::Dictionary,
            PropertyTreeValue::SignedInteger(_) => PropertyTreeType::SignedInteger,
            PropertyTreeValue::UnsignedInteger(_) => PropertyTreeType::UnsignedInteger,
        };

        writer.write_u8(type_val as u8)?;
        writer.write_u8(self.any_type as u8)?;

        match &self.value {
            PropertyTreeValue::Null => {}
            PropertyTreeValue::Bool(v) => {
                writer.write_u8(*v as u8)?;
            }
            PropertyTreeValue::Number(v) => {
                writer.write_f64::<LittleEndian>(*v)?;
            }
            PropertyTreeValue::String(v) => {
                v.save(writer)?;
            }
            PropertyTreeValue::List(items) => {
                writer.write_u32::<LittleEndian>(items.len() as u32)?;
                for item in items {
                    ImmutableString(None).save(writer)?;
                    item.save(writer)?;
                }
            }
            PropertyTreeValue::Dictionary(items) => {
                writer.write_u32::<LittleEndian>(items.len() as u32)?;
                for (key, item) in items {
                    key.save(writer)?;
                    item.save(writer)?;
                }
            }
            PropertyTreeValue::SignedInteger(v) => {
                writer.write_i64::<LittleEndian>(*v)?;
            }
            PropertyTreeValue::UnsignedInteger(v) => {
                writer.write_u64::<LittleEndian>(*v)?;
            }
        }
        Ok(())
    }
}

// Custom Serde for PropertyTree to match Python's JSON format.
impl Serialize for PropertyTree {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match &self.value {
            PropertyTreeValue::Null => serializer.serialize_none(),
            PropertyTreeValue::Bool(v) => serializer.serialize_bool(*v),
            PropertyTreeValue::Number(v) => serializer.serialize_f64(*v),
            PropertyTreeValue::String(v) => {
                if let Some(s) = v.to_string_lossy() {
                    serializer.serialize_str(&s)
                } else {
                    serializer.serialize_none()
                }
            }
            PropertyTreeValue::List(items) => items.serialize(serializer),
            PropertyTreeValue::Dictionary(items) => {
                use serde_json::Map;
                let mut map = Map::new();
                for (k, v) in items {
                    if let Some(ks) = k.to_string_lossy() {
                        map.insert(ks, serde_json::to_value(v).map_err(serde::ser::Error::custom)?);
                    }
                }
                map.serialize(serializer)
            }
            PropertyTreeValue::SignedInteger(v) => serializer.serialize_i64(*v),
            PropertyTreeValue::UnsignedInteger(v) => serializer.serialize_u64(*v),
        }
    }
}

impl<'de> Deserialize<'de> for PropertyTree {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        use serde_json::Value;
        let v = Value::deserialize(deserializer)?;
        Ok(Self::from_json_value(v))
    }
}

impl PropertyTree {
    fn from_json_value(v: serde_json::Value) -> Self {
        let value = match v {
            serde_json::Value::Null => PropertyTreeValue::Null,
            serde_json::Value::Bool(b) => PropertyTreeValue::Bool(b),
            serde_json::Value::Number(n) => {
                if n.is_i64() {
                    PropertyTreeValue::SignedInteger(n.as_i64().unwrap())
                } else if n.is_u64() {
                    PropertyTreeValue::UnsignedInteger(n.as_u64().unwrap())
                } else {
                    PropertyTreeValue::Number(n.as_f64().unwrap_or(0.0))
                }
            }
            serde_json::Value::String(s) => {
                PropertyTreeValue::String(ImmutableString::from_string(s))
            }
            serde_json::Value::Array(arr) => {
                PropertyTreeValue::List(arr.into_iter().map(Self::from_json_value).collect())
            }
            serde_json::Value::Object(obj) => {
                let items = obj
                    .into_iter()
                    .map(|(k, v)| (ImmutableString::from_string(k), Self::from_json_value(v)))
                    .collect();
                PropertyTreeValue::Dictionary(items)
            }
        };
        Self {
            value,
            any_type: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Version(pub u16, pub u16, pub u16, pub u16);

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ModSettings {
    #[serde(rename = "!type")]
    pub type_name: String,
    pub version: Version,
    pub has_quality: bool,
    pub data: PropertyTree,
}

impl ModSettings {
    pub fn load<R: Read>(reader: &mut R) -> Result<Self> {
        let v0 = reader.read_u16::<LittleEndian>()?;
        let v1 = reader.read_u16::<LittleEndian>()?;
        let v2 = reader.read_u16::<LittleEndian>()?;
        let v3 = reader.read_u16::<LittleEndian>()?;
        let version = Version(v0, v1, v2, v3);

        let has_quality = reader.read_u8()? != 0;
        let data = PropertyTree::load(reader)?;

        if (v0, v1, v2, v3) < (0, 18, 0, 0) {
            return Err(anyhow!(
                "Cannot load settings from Factorio {:?}: settings version too low",
                version
            ));
        }

        Ok(Self {
            type_name: "ModSettings".to_string(),
            version,
            has_quality,
            data,
        })
    }

    pub fn save<W: Write>(&self, writer: &mut W) -> Result<()> {
        writer.write_u16::<LittleEndian>(self.version.0)?;
        writer.write_u16::<LittleEndian>(self.version.1)?;
        writer.write_u16::<LittleEndian>(self.version.2)?;
        writer.write_u16::<LittleEndian>(self.version.3)?;
        writer.write_u8(self.has_quality as u8)?;
        self.data.save(writer)?;
        Ok(())
    }
}

impl<'de> Deserialize<'de> for ModSettings {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct RawModSettings {
            #[serde(rename = "!type")]
            type_name: String,
            version: Version,
            has_quality: bool,
            data: PropertyTree,
        }
        let raw = RawModSettings::deserialize(deserializer)?;
        if raw.type_name != "ModSettings" {
            return Err(serde::de::Error::custom(format!(
                "Unknown object type {}",
                raw.type_name
            )));
        }
        Ok(Self {
            type_name: raw.type_name,
            version: raw.version,
            has_quality: raw.has_quality,
            data: raw.data,
        })
    }
}

impl ModSettings {
    /// Convert raw `mod-settings.dat` bytes to a pretty-printed JSON string.
    pub fn dat_to_json(bytes: &[u8]) -> Result<String> {
        let mut r = std::io::Cursor::new(bytes);
        let s = Self::load(&mut r)?;
        Ok(serde_json::to_string_pretty(&s)?)
    }

    /// Convert a JSON string (as produced by `dat_to_json`) back to raw `.dat` bytes.
    pub fn json_to_dat(json: &str) -> Result<Vec<u8>> {
        let s: Self = serde_json::from_str(json)?;
        let mut w = Vec::new();
        s.save(&mut w)?;
        Ok(w)
    }

    /// Convert raw `mod-settings.dat` bytes to a YAML string.
    pub fn dat_to_yaml(bytes: &[u8]) -> Result<String> {
        let mut r = std::io::Cursor::new(bytes);
        let s = Self::load(&mut r)?;
        Ok(serde_yml::to_string(&s)?)
    }

    /// Convert a YAML string (as produced by `dat_to_yaml`) back to raw `.dat` bytes.
    pub fn yaml_to_dat(yaml: &str) -> Result<Vec<u8>> {
        let s: Self = serde_yml::from_str(yaml)?;
        let mut w = Vec::new();
        s.save(&mut w)?;
        Ok(w)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_bytes() -> Option<Vec<u8>> {
        let dirs = [
            concat!(env!("CARGO_MANIFEST_DIR"), "/mod-settings.dat"),
        ];
        for d in dirs {
            if let Ok(b) = std::fs::read(d) {
                return Some(b);
            }
        }
        None
    }

    #[test]
    fn test_dat_to_json_round_trip_byte_identical() {
        let bytes = sample_bytes().expect("sample mod-settings.dat must exist");
        let json = ModSettings::dat_to_json(&bytes).expect("dat -> json");
        let round = ModSettings::json_to_dat(&json).expect("json -> dat");
        assert_eq!(bytes, round, "dat -> json -> dat must be byte-identical");
    }

    #[test]
    fn test_dat_to_yaml_round_trip_byte_identical() {
        let bytes = sample_bytes().expect("sample mod-settings.dat must exist");
        let yaml = ModSettings::dat_to_yaml(&bytes).expect("dat -> yaml");
        let round = ModSettings::yaml_to_dat(&yaml).expect("yaml -> dat");
        assert_eq!(bytes, round, "dat -> yaml -> dat must be byte-identical");
    }
}
