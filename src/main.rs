use anyhow::{Context, Result, anyhow};
use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use clap::Parser;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fs::File;
use std::io::{BufReader, BufWriter, Read, Write};
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq)]
pub struct ImmutableString(pub Option<Vec<u8>>);

impl ImmutableString {
    fn load<R: Read>(reader: &mut R) -> Result<Self> {
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

    fn save<W: Write>(&self, writer: &mut W) -> Result<()> {
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

    fn from_string(s: String) -> Self {
        Self(Some(s.into_bytes()))
    }

    fn to_string_lossy(&self) -> Option<String> {
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
    fn load<R: Read>(reader: &mut R) -> Result<Self> {
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

    fn save<W: Write>(&self, writer: &mut W) -> Result<()> {
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
    fn load<R: Read>(reader: &mut R) -> Result<Self> {
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

    fn save<W: Write>(&self, writer: &mut W) -> Result<()> {
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

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    input: PathBuf,
    output: Option<PathBuf>,
}

fn main() -> Result<()> {
    let args = Args::parse();

    let input_path = &args.input;
    let input_name = input_path
        .file_name()
        .and_then(|n| n.to_str())
        .context("Invalid input filename")?;

    let mod_settings: ModSettings;

    if input_name.ends_with(".dat") {
        eprintln!("Reading DAT file '{:?}'...", input_path);
        let mut file = BufReader::new(File::open(input_path)?);
        mod_settings = ModSettings::load(&mut file)?;
    } else if input_name.ends_with(".json") {
        eprintln!("Reading JSON file '{:?}'...", input_path);
        let file = File::open(input_path)?;
        mod_settings = serde_json::from_reader(file)?;
    } else if input_name.ends_with(".yaml") || input_name.ends_with(".yml") {
        eprintln!("Reading YAML file '{:?}'...", input_path);
        let file = File::open(input_path)?;
        mod_settings = serde_yml::from_reader(file)?;
    } else {
        return Err(anyhow!(
            "Input filename '{:?}' does not end with .dat, .json, or .yaml/.yml.",
            input_path
        ));
    }

    let output_path = args.output.unwrap_or_else(|| {
        if input_name.ends_with(".dat") {
            input_path.with_extension("json")
        } else {
            input_path.with_extension("dat")
        }
    });

    let output_name = output_path
        .to_str()
        .context("Invalid output path")?;

    if output_name.ends_with(".dat") {
        eprintln!("Writing DAT file '{:?}'...", output_path);
        let mut file = BufWriter::new(File::create(&output_path)?);
        mod_settings.save(&mut file)?;
    } else if output_name.ends_with(".json") {
        eprintln!("Writing JSON file '{:?}'...", output_path);
        let file = File::create(&output_path)?;
        serde_json::to_writer_pretty(file, &mod_settings)?;
    } else if output_name.ends_with(".yaml") || output_name.ends_with(".yml") {
        eprintln!("Writing YAML file '{:?}'...", output_path);
        let file = File::create(&output_path)?;
        serde_yml::to_writer(file, &mod_settings)?;
    } else {
        return Err(anyhow!(
            "Output filename '{:?}' does not end with .dat, .json, or .yaml/.yml.",
            output_path
        ));
    }

    Ok(())
}
