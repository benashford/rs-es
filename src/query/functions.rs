/*
 * Copyright 2016-2018 Ben Ashford
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *     http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */

//! Specific options for the Function option of various queries

use std::collections::HashMap;

use serde::{Serialize, Serializer};

use crate::{
    json::{FieldBased, NoOuter, ShouldSkip},
    units::{Distance, Duration, JsonVal, Location},
};

/// Function
#[derive(Debug, Serialize)]
pub enum Function {
    #[serde(rename = "script_score")]
    ScriptScore(ScriptScore),
    #[serde(rename = "weight")]
    Weight(Weight),
    #[serde(rename = "random_score")]
    RandomScore(RandomScore),
    #[serde(rename = "field_value_factor")]
    FieldValueFactor(FieldValueFactor),
    #[serde(rename = "linear")]
    Linear(Decay),
    #[serde(rename = "exp")]
    Exp(Decay),
    #[serde(rename = "gauss")]
    Gauss(Decay),
}

/// ScriptScore function
#[derive(Debug, Default, Serialize)]
pub struct ScriptScore {
    #[serde(skip_serializing_if = "ShouldSkip::should_skip")]
    lang: Option<String>,
    params: HashMap<String, JsonVal>,
    inline: String,
}

impl Function {
    pub fn build_script_score<A>(script: A) -> ScriptScore
    where
        A: Into<String>,
    {
        ScriptScore {
            inline: script.into(),
            ..Default::default()
        }
    }
}

impl ScriptScore {
    add_field!(with_lang, lang, String);

    pub fn with_params<A>(mut self, params: A) -> Self
    where
        A: IntoIterator<Item = (String, JsonVal)>,
    {
        self.params.extend(params);
        self
    }

    pub fn add_param<A, B>(mut self, key: A, value: B) -> Self
    where
        A: Into<String>,
        B: Into<JsonVal>,
    {
        self.params.insert(key.into(), value.into());
        self
    }

    pub fn build(self) -> Function {
        Function::ScriptScore(self)
    }
}

/// Weight function
#[derive(Debug, Default, Serialize)]
pub struct Weight(f64);

impl Function {
    pub fn build_weight<A>(weight: A) -> Weight
    where
        A: Into<f64>,
    {
        Weight(weight.into())
    }
}

impl Weight {
    pub fn build(self) -> Function {
        Function::Weight(self)
    }
}

/// Random score function
#[derive(Debug, Default, Serialize)]
pub struct RandomScore(i64);

impl Function {
    pub fn build_random_score<A>(seed: A) -> RandomScore
    where
        A: Into<i64>,
    {
        RandomScore(seed.into())
    }
}

impl RandomScore {
    pub fn build(self) -> Function {
        Function::RandomScore(self)
    }
}

/// Field value factor function
#[derive(Debug, Default, Serialize)]
pub struct FieldValueFactor {
    field: String,
    #[serde(skip_serializing_if = "ShouldSkip::should_skip")]
    factor: Option<f64>,
    #[serde(skip_serializing_if = "ShouldSkip::should_skip")]
    modifier: Option<Modifier>,
    #[serde(skip_serializing_if = "ShouldSkip::should_skip")]
    missing: Option<JsonVal>,
}

impl Function {
    pub fn build_field_value_factor<A>(field: A) -> FieldValueFactor
    where
        A: Into<String>,
    {
        FieldValueFactor {
            field: field.into(),
            ..Default::default()
        }
    }
}

impl FieldValueFactor {
    add_field!(with_factor, factor, f64);
    add_field!(with_modifier, modifier, Modifier);
    add_field!(with_missing, missing, JsonVal);

    pub fn build(self) -> Function {
        Function::FieldValueFactor(self)
    }
}

/// Modifier for the FieldValueFactor function
#[derive(Debug)]
pub enum Modifier {
    None,
    Log,
    Log1p,
    Log2p,
    Ln,
    Ln1p,
    Ln2p,
    Square,
    Sqrt,
    Reciprocal,
}

impl Serialize for Modifier {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            Modifier::None => "none".serialize(serializer),
            Modifier::Log => "log".serialize(serializer),
            Modifier::Log1p => "log1p".serialize(serializer),
            Modifier::Log2p => "log2p".serialize(serializer),
            Modifier::Ln => "ln".serialize(serializer),
            Modifier::Ln1p => "ln1p".serialize(serializer),
            Modifier::Ln2p => "ln2p".serialize(serializer),
            Modifier::Square => "square".serialize(serializer),
            Modifier::Sqrt => "sqrt".serialize(serializer),
            Modifier::Reciprocal => "reciprocal".serialize(serializer),
        }
    }
}

#[derive(Debug, Default, Serialize)]
pub struct DecayOptions {
    origin: Origin,
    scale: Scale,
    #[serde(skip_serializing_if = "ShouldSkip::should_skip")]
    offset: Option<Scale>,
    #[serde(skip_serializing_if = "ShouldSkip::should_skip")]
    decay: Option<f64>,
    #[serde(skip_serializing_if = "ShouldSkip::should_skip")]
    multi_value_mode: Option<MultiValueMode>,
}

impl DecayOptions {
    pub fn new<A, B>(origin: A, scale: B) -> DecayOptions
    where
        A: Into<Origin>,
        B: Into<Scale>,
    {
        DecayOptions {
            origin: origin.into(),
            scale: scale.into(),
            offset: None,
            decay: None,
            multi_value_mode: None,
        }
    }

    add_field!(with_offset, offset, Scale);
    add_field!(with_decay, decay, f64);
    add_field!(with_multi_value_mode, multi_value_mode, MultiValueMode);

    pub fn with_scale(mut self, val: Scale) -> Self {
        self.scale = val;
        self
    }

    pub fn with_origin(mut self, val: Origin) -> Self {
        self.origin = val;
        self
    }

    pub fn build<A: Into<String>>(self, field: A) -> Decay {
        Decay(FieldBased::new(
            field.into(),
            self,
            NoOuter,
        ))
    }
}

/// Decay functions
#[derive(Debug, Serialize)]
pub struct Decay(FieldBased<String, DecayOptions, NoOuter>);

impl Function {
    pub fn build_decay<A, B, C>(field: A, origin: B, scale: C) -> Decay
    where
        A: Into<String>,
        B: Into<Origin>,
        C: Into<Scale>,
    {
        Decay(FieldBased::new(
            field.into(),
            DecayOptions {
                origin: origin.into(),
                scale: scale.into(),
                ..Default::default()
            },
            NoOuter,
        ))
    }

    pub fn build_decay_from_options<A: Into<String>>(field: A, options: DecayOptions) -> Decay {
        options.build(field)
    }
}

impl Decay {
    pub fn build_linear(self) -> Function {
        Function::Linear(self)
    }

    pub fn build_exp(self) -> Function {
        Function::Exp(self)
    }

    pub fn build_gauss(self) -> Function {
        Function::Gauss(self)
    }
}

// options used by decay functions

/// Origin for decay function
#[derive(Debug)]
pub enum Origin {
    I64(i64),
    U64(u64),
    F64(f64),
    Location(Location),
    Date(String),
}

impl Default for Origin {
    fn default() -> Origin {
        Origin::I64(0)
    }
}

from!(i64, Origin, I64);
from!(u64, Origin, U64);
from!(f64, Origin, F64);
from!(Location, Origin, Location);
from!(String, Origin, Date);

impl Serialize for Origin {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            Origin::I64(orig) => orig.serialize(serializer),
            Origin::U64(orig) => orig.serialize(serializer),
            Origin::F64(orig) => orig.serialize(serializer),
            Origin::Location(ref orig) => orig.serialize(serializer),
            Origin::Date(ref orig) => orig.serialize(serializer),
        }
    }
}

/// Scale used by decay function
#[derive(Debug)]
pub enum Scale {
    I64(i64),
    U64(u64),
    F64(f64),
    Distance(Distance),
    Duration(Duration),
}

impl Default for Scale {
    fn default() -> Self {
        Scale::I64(0)
    }
}

from!(i64, Scale, I64);
from!(u64, Scale, U64);
from!(f64, Scale, F64);
from!(Distance, Scale, Distance);
from!(Duration, Scale, Duration);

impl Serialize for Scale {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            Scale::I64(s) => s.serialize(serializer),
            Scale::U64(s) => s.serialize(serializer),
            Scale::F64(s) => s.serialize(serializer),
            Scale::Distance(ref s) => s.serialize(serializer),
            Scale::Duration(ref s) => s.serialize(serializer),
        }
    }
}

/// Values for multi_value_mode
#[derive(Debug)]
pub enum MultiValueMode {
    Min,
    Max,
    Avg,
    Sum,
}

impl Serialize for MultiValueMode {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        use self::MultiValueMode::*;
        match self {
            Min => "min",
            Max => "max",
            Avg => "avg",
            Sum => "sum",
        }
        .serialize(serializer)
    }
}

#[cfg(test)]
pub mod tests {
    use serde_json;

    #[test]
    fn test_decay_query() {
        use crate::units::*;
        let gauss_decay_query = super::Function::build_decay(
            "my_field",
            Location::LatLon(42., 24.),
            Distance::new(3., DistanceUnit::Kilometer),
        )
        .build_gauss();

        assert_eq!(
            r#"{"gauss":{"my_field":{"origin":{"lat":42.0,"lon":24.0},"scale":"3km"}}}"#,
            serde_json::to_string(&gauss_decay_query).unwrap()
        );
    }
}
