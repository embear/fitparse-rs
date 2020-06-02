//! # FitParser
//!
//! `fitparser` is a utility to parse an ANT FIT file based on a given profile into a more
//! useful form for consuming applications. To that end the [serde](https://github.com/serde-rs/serde)
//! framework is used to allow the data to be serialized into any format supported by serde. This
//! library currently does not support writing FIT files.
//!
//! ## Example
//! Open a file or pass in any other object that implements the Read
//! trait. FIT files can be chained so a single file can contain more than
//! one dataset which is why `parse` returns a Vec.
//! ```
//! use fitparser;
//! use std::fs::File;
//! use std::io::prelude::*;
//!
//! let mut fp = File::open("tests/fixtures/Activity.fit")?;
//! for data in fitparser::from_reader(&mut fp)? {
//!     // print the data in FIT file
//!     println!("{:#?}", data);
//!     // alternatively reserialize the data into a new format with serde
//!     // println!("{:#?}",  serde_json::to_string(data)?);
//! }
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
#![warn(missing_docs)]
use chrono::{DateTime, Local};
use serde::Serialize;
use std::convert;
use std::fmt;

mod de;
mod error;
pub mod profile;
pub mod ser;

pub use de::{from_bytes, from_reader, Deserializer};
pub use error::{Error, ErrorKind, Result};
use ser::{FitDataRecordSerializer, ValueWithUnits};

/// Defines a set of data derived from a FIT Data message.
#[derive(Clone, Debug, Serialize)]
pub struct FitDataRecord {
    /// The kind of message the data came from, the FIT profile defines several messages and
    /// custom messages can be defined by altering the profile
    kind: profile::MesgNum,
    /// All the fields present in this message, a record may not have every possible field defined
    fields: Vec<FitDataField>,
}

impl FitDataRecord {
    /// Create an empty data record with a given kind
    pub fn new(kind: profile::MesgNum) -> Self {
        FitDataRecord {
            kind,
            fields: Vec::new(),
        }
    }

    /// Return the kind of FitDataRecord, this value is defined by the FIT profile.
    pub fn kind(&self) -> profile::MesgNum {
        self.kind
    }

    /// Fetch a field from the record
    pub fn fields(&self) -> &[FitDataField] {
        &self.fields
    }

    /// Add a field to the record
    pub fn push(&mut self, field: FitDataField) {
        self.fields.push(field)
    }

    /// Restructure the record so fields are accessed by their definition number and values are
    /// stored without the units defined in the FIT profile. This conscise format assumes the
    /// consumer knows the FIT profile in use or uses the data in a way that it doesn't need to
    /// know about the FIT profile.
    pub fn into_number_key_plain_value_mapping(self) -> FitDataRecordSerializer<u16, u8, Value> {
        let mut record_ser = FitDataRecordSerializer::new(self.kind.as_u16());
        for field in self.fields {
            record_ser.insert(field.number, field.value);
        }
        record_ser
    }

    /// Same as the `into_number_key_plain_value_mapping` function except each value is stored
    /// with the units defined by the FIT profile (if any)
    pub fn into_number_key_value_with_units_mapping(
        self,
    ) -> FitDataRecordSerializer<u16, u8, ValueWithUnits> {
        let mut record_ser = FitDataRecordSerializer::new(self.kind.as_u16());
        for field in self.fields {
            record_ser.insert(field.number, ValueWithUnits::new(field.value, field.units));
        }
        record_ser
    }

    /// Restructure the record so fields are accessed by their `name`, this is preferable if the
    /// consumer is not aware of the defined FIT profile and therefore cannot decode the name from
    /// the message number + definition number combination. Values are provided without units.
    pub fn into_name_key_plain_value_mapping(
        self,
    ) -> FitDataRecordSerializer<String, String, Value> {
        let mut record_ser = FitDataRecordSerializer::new(self.kind.to_string());
        for field in self.fields {
            record_ser.insert(field.name, field.value);
        }
        record_ser
    }

    /// Same as the `into_name_key_plain_value_mapping` function, except each value is stored with
    /// the units defined by the FIT profile (if any). This is the most verbose format and is ideal
    /// when the consumer has no knowledge of the FIT profile in use.
    pub fn into_name_key_value_with_units_mapping(
        self,
    ) -> FitDataRecordSerializer<String, String, ValueWithUnits> {
        let mut record_ser = FitDataRecordSerializer::new(self.kind.to_string());
        for field in self.fields {
            record_ser.insert(field.name, ValueWithUnits::new(field.value, field.units));
        }
        record_ser
    }
}

/// Stores a value and it's defined units which are set by the FIT profile during decoding
#[derive(Clone, Debug, Serialize)]
pub struct FitDataField {
    name: String,
    number: u8,
    value: Value,
    units: String,
}

impl FitDataField {
    /// Create a new FitDataField
    pub fn new(name: String, number: u8, value: Value, units: String) -> Self {
        FitDataField {
            name,
            number,
            value,
            units,
        }
    }

    /// Return stored value
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Return stored value
    pub fn number(&self) -> u8 {
        self.number
    }

    /// Return stored value
    pub fn value(&self) -> &Value {
        &self.value
    }

    /// Return units associated with the value
    pub fn units(&self) -> &str {
        &self.units
    }
}

impl fmt::Display for FitDataField {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.units.is_empty() {
            write!(f, "{}", self.value)
        } else {
            write!(f, "{} {}", self.value, self.units)
        }
    }
}

/// Contains arbitrary data in the defined format. These types are condensed from the full list of
/// possible types defined by the FIT profile
#[derive(Clone, Debug, PartialEq, PartialOrd, Serialize)]
#[serde(untagged)]
pub enum Value {
    /// Timestamp field converted to the local timezone
    Timestamp(DateTime<Local>),
    /// Unsigned 8bit integer data
    Byte(u8),
    /// Unsigned 8bit integer that gets mapped to a FieldType enum
    Enum(u8),
    /// Signed 8bit integer data
    SInt8(i8),
    /// Unsigned 8bit integer data
    UInt8(u8),
    /// Signed 16bit integer data
    SInt16(i16),
    /// Unsigned 16bit integer data
    UInt16(u16),
    /// Signed 32bit integer data
    SInt32(i32),
    /// Unsigned 32bit integer data
    UInt32(u32),
    /// UTF-8 format string data
    String(String),
    /// 32bit floating point data
    Float32(f32),
    /// 64bit floating point data
    Float64(f64),
    /// Unsigned 8bit integer data where the invalid value is 0x0 instead of 0xFF
    UInt8z(u8),
    /// Unsigned 16bit integer data where the invalid value is 0x0 instead of 0xFFFF
    UInt16z(u16),
    /// Unsigned 16bit integer data where the invalid value is 0x0 instead of 0xFFFFFFFF
    UInt32z(u32),
    /// Signed 64bit integer data
    SInt64(i64),
    /// Unsigned 64bit integer data
    UInt64(u64),
    /// Unsigned 64bit integer data where the invalid value is 0x0 instead of 0xFFFFFFFFFFFFFFFF
    UInt64z(u64),
    /// Array of DataFitDataField, while this allows nested arrays and mixed types this is not possible
    /// in a properly formatted FIT file
    Array(Vec<Self>),
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self {
            Value::Timestamp(val) => write!(f, "{}", val),
            Value::Byte(val) => write!(f, "{}", val),
            Value::Enum(val) => write!(f, "{}", val),
            Value::SInt8(val) => write!(f, "{}", val),
            Value::UInt8(val) => write!(f, "{}", val),
            Value::UInt8z(val) => write!(f, "{}", val),
            Value::SInt16(val) => write!(f, "{}", val),
            Value::UInt16(val) => write!(f, "{}", val),
            Value::UInt16z(val) => write!(f, "{}", val),
            Value::SInt32(val) => write!(f, "{}", val),
            Value::UInt32(val) => write!(f, "{}", val),
            Value::UInt32z(val) => write!(f, "{}", val),
            Value::SInt64(val) => write!(f, "{}", val),
            Value::UInt64(val) => write!(f, "{}", val),
            Value::UInt64z(val) => write!(f, "{}", val),
            Value::Float32(val) => write!(f, "{}", val),
            Value::Float64(val) => write!(f, "{}", val),
            Value::String(val) => write!(f, "{}", val),
            Value::Array(vals) => write!(f, "{:?}", vals), // printing arrays is hard
        }
    }
}

impl convert::TryInto<f64> for Value {
    type Error = error::Error;

    fn try_into(self) -> Result<f64> {
        match self {
            Value::Timestamp(val) => Ok(val.timestamp() as f64),
            Value::Byte(val) => Ok(val as f64),
            Value::Enum(val) => Ok(val as f64),
            Value::SInt8(val) => Ok(val as f64),
            Value::UInt8(val) => Ok(val as f64),
            Value::UInt8z(val) => Ok(val as f64),
            Value::SInt16(val) => Ok(val as f64),
            Value::UInt16(val) => Ok(val as f64),
            Value::UInt16z(val) => Ok(val as f64),
            Value::SInt32(val) => Ok(val as f64),
            Value::UInt32(val) => Ok(val as f64),
            Value::UInt32z(val) => Ok(val as f64),
            Value::SInt64(val) => Ok(val as f64),
            Value::UInt64(val) => Ok(val as f64),
            Value::UInt64z(val) => Ok(val as f64),
            Value::Float32(val) => Ok(val as f64),
            Value::Float64(val) => Ok(val),
            Value::String(_) => {
                Err(ErrorKind::ValueError(format!("cannot convert {} into an f64", self)).into())
            }
            Value::Array(_) => {
                Err(ErrorKind::ValueError(format!("cannot convert {} into an f64", self)).into())
            }
        }
    }
}

impl convert::TryInto<i64> for Value {
    type Error = error::Error;

    fn try_into(self) -> Result<i64> {
        match self {
            Value::Timestamp(val) => Ok(val.timestamp()),
            Value::Byte(val) => Ok(val as i64),
            Value::Enum(val) => Ok(val as i64),
            Value::SInt8(val) => Ok(val as i64),
            Value::UInt8(val) => Ok(val as i64),
            Value::UInt8z(val) => Ok(val as i64),
            Value::SInt16(val) => Ok(val as i64),
            Value::UInt16(val) => Ok(val as i64),
            Value::UInt16z(val) => Ok(val as i64),
            Value::SInt32(val) => Ok(val as i64),
            Value::UInt32(val) => Ok(val as i64),
            Value::UInt32z(val) => Ok(val as i64),
            Value::SInt64(val) => Ok(val),
            Value::UInt64(val) => Ok(val as i64),
            Value::UInt64z(val) => Ok(val as i64),
            Value::Float32(_) => {
                Err(ErrorKind::ValueError(format!("cannot convert {} into an i64", self)).into())
            }
            Value::Float64(_) => {
                Err(ErrorKind::ValueError(format!("cannot convert {} into an i64", self)).into())
            }
            Value::String(_) => {
                Err(ErrorKind::ValueError(format!("cannot convert {} into an i64", self)).into())
            }
            Value::Array(_) => {
                Err(ErrorKind::ValueError(format!("cannot convert {} into an i64", self)).into())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_activity() {
        let data = include_bytes!("../tests/fixtures/Activity.fit").to_vec();
        let fit_data = from_bytes(&data).unwrap();
        assert_eq!(fit_data.len(), 22);
    }

    #[test]
    fn parse_developer_data() {
        let data = include_bytes!("../tests/fixtures/DeveloperData.fit").to_vec();
        let fit_data = from_bytes(&data).unwrap();
        assert_eq!(fit_data.len(), 6);
    }

    #[test]
    fn parse_monitoring_file() {
        let data = include_bytes!("../tests/fixtures/MonitoringFile.fit").to_vec();
        let fit_data = from_bytes(&data).unwrap();
        assert_eq!(fit_data.len(), 355);
    }

    #[test]
    fn parse_settings() {
        let data = include_bytes!("../tests/fixtures/Settings.fit").to_vec();
        let fit_data = from_bytes(&data).unwrap();
        assert_eq!(fit_data.len(), 3);
    }

    #[test]
    fn parse_weight_scale_multi_user() {
        let data = include_bytes!("../tests/fixtures/WeightScaleMultiUser.fit").to_vec();
        let fit_data = from_bytes(&data).unwrap();
        assert_eq!(fit_data.len(), 7);
    }

    #[test]
    fn parse_weight_scale_single_user() {
        let data = include_bytes!("../tests/fixtures/WeightScaleSingleUser.fit").to_vec();
        let fit_data = from_bytes(&data).unwrap();
        assert_eq!(fit_data.len(), 6);
    }

    #[test]
    fn parse_workout_custom_target_values() {
        let data = include_bytes!("../tests/fixtures/WorkoutCustomTargetValues.fit").to_vec();
        let fit_data = from_bytes(&data).unwrap();
        assert_eq!(fit_data.len(), 6);
    }

    #[test]
    fn parse_workout_individual_steps() {
        let data = include_bytes!("../tests/fixtures/WorkoutIndividualSteps.fit").to_vec();
        let fit_data = from_bytes(&data).unwrap();
        assert_eq!(fit_data.len(), 6);
    }

    #[test]
    fn parse_workout_repeat_greater_than_step() {
        let data = include_bytes!("../tests/fixtures/WorkoutRepeatGreaterThanStep.fit").to_vec();
        let fit_data = from_bytes(&data).unwrap();
        assert_eq!(fit_data.len(), 7);
    }

    #[test]
    fn parse_workout_repeat_steps() {
        let data = include_bytes!("../tests/fixtures/WorkoutRepeatSteps.fit").to_vec();
        let fit_data = from_bytes(&data).unwrap();
        assert_eq!(fit_data.len(), 7);
    }

    #[test]
    fn parse_garmin_fenix_5_bike() {
        // this test case includes a FIT file with a string field, which was broken in v0.1.0
        // and fixed in v0.1.1
        let data = include_bytes!("../tests/fixtures/garmin-fenix-5-bike.fit").to_vec();
        let fit_data = from_bytes(&data).unwrap();
        assert_eq!(fit_data.len(), 143);
    }

    #[test]
    fn parse_sample_mulitple_header() {
        // this test case includes a chained FIT file
        let data = include_bytes!("../tests/fixtures/sample_mulitple_header.fit").to_vec();
        let fit_data = from_bytes(&data).unwrap();
        assert_eq!(fit_data.len(), 3023);
    }
}
