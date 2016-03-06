/*
 * Copyright 2016 Ben Ashford
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

use std::collections::{BTreeMap, HashMap};

use rustc_serialize::json::{Json, ToJson};

use ::units::{Distance, Duration, JsonVal, Location};

/// Function
#[derive(Debug)]
pub enum Function {
    ScriptScore(ScriptScore),
    Weight(Weight),
    RandomScore(RandomScore),
    FieldValueFactor(FieldValueFactor),
    Decay(Decay)
}

impl ToJson for Function {
    fn to_json(&self) -> Json {
        let mut d = BTreeMap::new();
        match self {
            &Function::ScriptScore(ref q) => {
                d.insert("script_score".to_owned(), q.to_json());
            },
            &Function::Weight(ref q) => {
                d.insert("weight".to_owned(), q.to_json());
            },
            &Function::RandomScore(ref q) => {
                d.insert("random_score".to_owned(), q.to_json());
            },
            &Function::FieldValueFactor(ref q) => {
                d.insert("field_value_factor".to_owned(), q.to_json());
            },
            &Function::Decay(ref q) => {
                q.add_to_json(&mut d);
            }
        }
        Json::Object(d)
    }
}

/// ScriptScore function
#[derive(Debug, Default)]
pub struct ScriptScore {
    lang: Option<String>,
    params: HashMap<String, JsonVal>,
    inline: String
}

impl Function {
    pub fn build_script_score<A>(script: A) -> ScriptScore
        where A: Into<String> {

        ScriptScore {
            inline: script.into(),
            ..Default::default()
        }
    }
}

impl ScriptScore {
    add_option!(with_lang, lang, String);

    pub fn with_params<A>(mut self, params: A) -> Self
        where A: IntoIterator<Item=(String, JsonVal)> {

        self.params.extend(params);
        self
    }

    pub fn add_param<A, B>(mut self, key: A, value: B) -> Self
        where A: Into<String>,
              B: Into<JsonVal> {
        self.params.insert(key.into(), value.into());
        self
    }

    pub fn build(self) -> Function {
        Function::ScriptScore(self)
    }
}

impl ToJson for ScriptScore {
    fn to_json(&self) -> Json {
        let mut d = BTreeMap::new();
        d.insert("inline".to_owned(), self.inline.to_json());
        d.insert("params".to_owned(), self.params.to_json());
        optional_add!(self, d, lang);
        Json::Object(d)
    }
}

/// Weight function
#[derive(Debug, Default)]
pub struct Weight(f64);

impl Function {
    pub fn build_weight<A>(weight: A) -> Weight
        where A: Into<f64> {

        Weight(weight.into())
    }
}

impl Weight {
    pub fn build(self) -> Function {
        Function::Weight(self)
    }
}

impl ToJson for Weight {
    fn to_json(&self) -> Json {
        self.0.to_json()
    }
}

/// Random score function
#[derive(Debug, Default)]
pub struct RandomScore(i64);

impl Function {
    pub fn build_random_score<A>(seed: A) -> RandomScore
        where A: Into<i64> {

        RandomScore(seed.into())
    }
}

impl RandomScore {
    pub fn build(self) -> Function {
        Function::RandomScore(self)
    }
}

impl ToJson for RandomScore {
    fn to_json(&self) -> Json {
        self.0.to_json()
    }
}

/// Field value factor function
#[derive(Debug, Default)]
pub struct FieldValueFactor {
    field: String,
    factor: Option<f64>,
    modifier: Option<Modifier>,
    missing: Option<JsonVal>
}

impl Function {
    pub fn build_field_value_factor<A>(field: A) -> FieldValueFactor
        where A: Into<String> {

        FieldValueFactor {
            field: field.into(),
            ..Default::default()
        }
    }
}

impl FieldValueFactor {
    add_option!(with_factor, factor, f64);
    add_option!(with_modifier, modifier, Modifier);
    add_option!(with_missing, missing, JsonVal);

    pub fn build(self) -> Function {
        Function::FieldValueFactor(self)
    }
}

impl ToJson for FieldValueFactor {
    fn to_json(&self) -> Json {
        let mut d = BTreeMap::new();
        d.insert("field".to_owned(), self.field.to_json());
        optional_add!(self, d, factor);
        optional_add!(self, d, modifier);
        optional_add!(self, d, missing);
        Json::Object(d)
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

impl ToJson for Modifier {
    fn to_json(&self) -> Json {
        match self {
            &Modifier::None => "none".to_json(),
            &Modifier::Log => "log".to_json(),
            &Modifier::Log1p => "log1p".to_json(),
            &Modifier::Log2p => "log2p".to_json(),
            &Modifier::Ln => "ln".to_json(),
            &Modifier::Ln1p => "ln1p".to_json(),
            &Modifier::Ln2p => "ln2p".to_json(),
            &Modifier::Square => "square".to_json(),
            &Modifier::Sqrt => "sqrt".to_json(),
            &Modifier::Reciprocal => "reciprocal".to_json(),
        }
    }
}

// Decay functions
/// Type of decay functions
#[derive(Debug)]
pub enum DecayFunction {
    Linear,
    Exp,
    Gauss
}

impl Default for DecayFunction {
    fn default() -> Self {
        DecayFunction::Linear
    }
}

impl ToString for DecayFunction {
    fn to_string(&self) -> String {
        match self {
            &DecayFunction::Linear => "linear",
            &DecayFunction::Exp => "exp",
            &DecayFunction::Gauss => "gauss"
        }.to_owned()
    }
}

/// Decay functions
#[derive(Debug, Default)]
pub struct Decay {
    decay_function: DecayFunction,
    field: String,
    origin: Origin,
    scale: Scale,
    offset: Option<Scale>,
    decay: Option<f64>,
    multi_value_mode: Option<MultiValueMode>
}

macro_rules! build_decay_fn {
    ($fn_name:ident, $dec_fn:expr) => (
        pub fn $fn_name<A, B, C>(field: A, origin: B, scale: C) -> Decay
            where A: Into<String>,
                  B: Into<Origin>,
                  C: Into<Scale> {
            Decay {
                decay_function: $dec_fn,
                field: field.into(),
                origin: origin.into(),
                scale: scale.into(),
                ..Default::default()
            }
        }
    )
}

impl Function {
    build_decay_fn!(build_linear_decay, DecayFunction::Linear);
    build_decay_fn!(build_exp_decay, DecayFunction::Exp);
    build_decay_fn!(build_gauss_decay, DecayFunction::Gauss);
}

impl Decay {
    add_option!(with_offset, offset, Scale);
    add_option!(with_decay, decay, f64);
    add_option!(with_multi_value_mode, multi_value_mode, MultiValueMode);

    pub fn build(self) -> Function {
        Function::Decay(self)
    }

    pub fn add_to_json(&self, d: &mut BTreeMap<String, Json>) {
        let mut inner = BTreeMap::new();
        let mut params = BTreeMap::new();
        params.insert("origin".to_owned(), self.origin.to_json());
        params.insert("scale".to_owned(), self.scale.to_json());
        optional_add!(self, params, offset);
        optional_add!(self, params, decay);
        inner.insert(self.field.clone(), Json::Object(params));
        optional_add!(self, inner, multi_value_mode);
        d.insert(self.decay_function.to_string(), Json::Object(inner));
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
    Date(String)
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

impl ToJson for Origin {
    fn to_json(&self) -> Json {
        match self {
            &Origin::I64(orig)          => Json::I64(orig),
            &Origin::U64(orig)          => Json::U64(orig),
            &Origin::F64(orig)          => Json::F64(orig),
            &Origin::Location(ref orig) => orig.to_json(),
            &Origin::Date(ref orig)     => Json::String(orig.clone())
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
    Duration(Duration)
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

impl ToJson for Scale {
    fn to_json(&self) -> Json {
        match self {
            &Scale::I64(s)          => Json::I64(s),
            &Scale::U64(s)          => Json::U64(s),
            &Scale::F64(s)          => Json::F64(s),
            &Scale::Distance(ref s) => s.to_json(),
            &Scale::Duration(ref s) => s.to_json()
        }
    }
}

/// Values for multi_value_mode
#[derive(Debug)]
pub enum MultiValueMode {
    Min,
    Max,
    Avg,
    Sum
}

impl ToJson for MultiValueMode {
    fn to_json(&self) -> Json {
        match self {
            &MultiValueMode::Min => "min".to_json(),
            &MultiValueMode::Max => "max".to_json(),
            &MultiValueMode::Avg => "avg".to_json(),
            &MultiValueMode::Sum => "sum".to_json(),
        }
    }
}
