/*
 * Copyright 2015-2016 Ben Ashford
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

//! For metrics-based aggregations

use std::collections::{BTreeMap, HashMap};

use serde::ser;
use serde::ser::{Serialize, Serializer};
use serde_json::{to_value, Value};

// TODO - deprecated
use rustc_serialize::json::{Json, ToJson};

use ::units::JsonVal;

// TODO - deprecated
use super::FieldOrScript;

use super::Aggregation;

macro_rules! metrics_agg {
    ($b:ident) => {
        impl<'a> $b<'a> {
            pub fn field(field: &'a str) -> Self {
                $b(MetricAgg {
                    field: Some(field),
                    ..Default::default()
                })
            }

            pub fn script<S: Into<Script<'a>>>(script: S) -> Self {
                $b(MetricAgg {
                    script: script.into(),
                    ..Default::default()
                })
            }

            pub fn with_script<S: Into<Script<'a>>>(mut self, script: S) -> Self {
                self.0.script = script.into();
                self
            }

            pub fn with_missing<J: Into<JsonVal>>(mut self, missing: J) -> Self {
                self.0.missing = Some(missing.into());
                self
            }
        }

        impl<'a> From<$b<'a>> for Aggregation<'a> {
            fn from(from: $b<'a>) -> Aggregation<'a> {
                Aggregation::Metrics(MetricsAggregation::$b(from))
            }
        }

        impl<'a> Serialize for $b<'a> {
            fn serialize<S>(&self, serializer: &mut S) -> Result<(), S::Error>
                where S: Serializer {

                self.0.serialize(serializer)
            }
        }
    }
}

/// Scripts used in aggregations
#[derive(Debug, Default)]
struct Script<'a> {
    inline: Option<&'a str>,
    file: Option<&'a str>,
    id: Option<&'a str>,
    params: Option<HashMap<&'a str, JsonVal>>
}

/// Base of all Metrics aggregations
#[derive(Debug, Default)]
struct MetricAgg<'a> {
    field: Option<&'a str>,
    script: Script<'a>,
    missing: Option<JsonVal>
}

impl<'a> Serialize for MetricAgg<'a> {
    fn serialize<S>(&self, serializer: &mut S) -> Result<(), S::Error>
        where S: Serializer {

        serializer.serialize_struct("MetricAgg", MetricAggVisitor {
            ma: self,
            state: 0
        })
    }
}

struct MetricAggVisitor<'a> {
    ma: &'a MetricAgg<'a>,
    state: u8
}

fn visit_field<S, T>(field: Option<T>,
                     field_name: &str,
                     serializer: &mut S) -> Result<Option<()>, S::Error>
    where S: Serializer,
          T: Serialize {

    match field {
        Some(value) => {
            Ok(Some(try!(serializer.serialize_map_elt(field_name, value))))
        },
        None => Ok(Some(()))
    }
}

impl<'a> ser::MapVisitor for MetricAggVisitor<'a> {
    fn visit<S>(&mut self, serializer: &mut S) -> Result<Option<()>, S::Error>
        where S: Serializer {

        self.state += 1;
        match self.state {
            1 => visit_field(self.ma.field, "field", serializer),
            2 => visit_field(self.ma.script.inline, "inline", serializer),
            3 => visit_field(self.ma.script.file, "file", serializer),
            4 => visit_field(self.ma.script.id, "id", serializer),
            5 => visit_field(self.ma.script.params.as_ref(), "params", serializer),
            6 => visit_field(self.ma.missing.as_ref(), "missing", serializer),
            _ => Ok(None)
        }
    }
}

/// Min aggregation
#[derive(Debug)]
pub struct Min<'a>(MetricAgg<'a>);
metrics_agg!(Min);

// /// Max aggregation
// #[derive(Debug)]
// pub struct Max<'a>(FieldOrScript<'a>);

// field_or_script_new!(Max);
// field_or_script_to_json!(Max);
// metrics_agg!(Max);

// /// Sum aggregation
// #[derive(Debug)]
// pub struct Sum<'a>(FieldOrScript<'a>);

// field_or_script_new!(Sum);
// field_or_script_to_json!(Sum);
// metrics_agg!(Sum);

// /// Avg aggregation
// #[derive(Debug)]
// pub struct Avg<'a>(FieldOrScript<'a>);

// field_or_script_new!(Avg);
// field_or_script_to_json!(Avg);
// metrics_agg!(Avg);

// /// Stats aggregation
// #[derive(Debug)]
// pub struct Stats<'a>(FieldOrScript<'a>);

// field_or_script_new!(Stats);
// field_or_script_to_json!(Stats);
// metrics_agg!(Stats);

// /// Extended stats aggregation
// #[derive(Debug)]
// pub struct ExtendedStats<'a>(FieldOrScript<'a>);

// field_or_script_new!(ExtendedStats);
// field_or_script_to_json!(ExtendedStats);
// metrics_agg!(ExtendedStats);

// /// Value count aggregation
// #[derive(Debug)]
// pub struct ValueCount<'a>(FieldOrScript<'a>);

// field_or_script_new!(ValueCount);
// field_or_script_to_json!(ValueCount);
// metrics_agg!(ValueCount);

// /// Percentiles aggregation, see: https://www.elastic.co/guide/en/elasticsearch/reference/current/search-aggregations-metrics-percentile-aggregation.html
// ///
// /// # Examples
// ///
// /// ```
// /// use rs_es::operations::search::aggregations::Percentiles;
// ///
// /// let p1 = Percentiles::new("field_name").with_compression(100u64);
// /// let p2 = Percentiles::new("field_name").with_percents(vec![10.0, 20.0]);
// /// ```
// #[derive(Debug)]
// pub struct Percentiles<'a> {
//     fos:         FieldOrScript<'a>,
//     percents:    Option<Vec<f64>>,
//     compression: Option<u64>
// }

// impl<'a> Percentiles<'a> {
//     pub fn new<F: Into<FieldOrScript<'a>>>(fos: F) -> Percentiles<'a> {
//         Percentiles {
//             fos:         fos.into(),
//             percents:    None,
//             compression: None
//         }
//     }

//     add_field!(with_percents, percents, Vec<f64>);
//     add_field!(with_compression, compression, u64);
// }

// impl<'a> ToJson for Percentiles<'a> {
//     fn to_json(&self) -> Json {
//         let mut d = BTreeMap::new();
//         self.fos.add_to_object(&mut d);
//         optional_add!(self, d, percents);
//         optional_add!(self, d, compression);
//         Json::Object(d)
//     }
// }

// metrics_agg!(Percentiles);

// /// Percentile Ranks aggregation
// #[derive(Debug)]
// pub struct PercentileRanks<'a> {
//     fos:    FieldOrScript<'a>,
//     values: Vec<f64>
// }

// impl<'a> PercentileRanks<'a> {
//     pub fn new<F: Into<FieldOrScript<'a>>>(fos: F, vals: Vec<f64>) -> PercentileRanks<'a> {
//         PercentileRanks {
//             fos:    fos.into(),
//             values: vals
//         }
//     }
// }

// impl<'a> ToJson for PercentileRanks<'a> {
//     fn to_json(&self) -> Json {
//         let mut d = BTreeMap::new();
//         self.fos.add_to_object(&mut d);
//         d.insert("values".to_owned(), self.values.to_json());
//         Json::Object(d)
//     }
// }

// metrics_agg!(PercentileRanks);

// /// Cardinality aggregation
// #[derive(Debug)]
// pub struct Cardinality<'a> {
//     fos:                 FieldOrScript<'a>,
//     precision_threshold: Option<u64>,
//     rehash:              Option<bool>
// }

// impl<'a> Cardinality<'a> {
//     pub fn new<F: Into<FieldOrScript<'a>>>(fos: F) -> Cardinality<'a> {
//         Cardinality {
//             fos:                 fos.into(),
//             precision_threshold: None,
//             rehash:              None
//         }
//     }

//     add_field!(with_precision_threshold, precision_threshold, u64);
//     add_field!(with_rehash, rehash, bool);
// }

// impl<'a> ToJson for Cardinality<'a> {
//     fn to_json(&self) -> Json {
//         let mut d = BTreeMap::new();
//         self.fos.add_to_object(&mut d);
//         optional_add!(self, d, precision_threshold);
//         optional_add!(self, d, rehash);
//         Json::Object(d)
//     }
// }

// metrics_agg!(Cardinality);

// /// Geo Bounds aggregation
// #[derive(Debug)]
// pub struct GeoBounds<'a> {
//     field:          &'a str,
//     wrap_longitude: Option<bool>
// }

// impl<'a> GeoBounds<'a> {
//     pub fn new(field: &'a str) -> GeoBounds {
//         GeoBounds {
//             field: field,
//             wrap_longitude: None
//         }
//     }

//     add_field!(with_wrap_longitude, wrap_longitude, bool);
// }

// impl<'a> ToJson for GeoBounds<'a> {
//     fn to_json(&self) -> Json {
//         let mut d = BTreeMap::new();
//         d.insert("field".to_owned(), self.field.to_json());
//         optional_add!(self, d, wrap_longitude);
//         Json::Object(d)
//     }
// }

// metrics_agg!(GeoBounds);

/// Scripted method aggregation
#[derive(Debug)]
pub struct ScriptedMetric<'a> {
    init_script:         Option<&'a str>,
    map_script:          &'a str,
    combine_script:      Option<&'a str>,
    reduce_script:       Option<&'a str>,
    params:              Option<Json>,
    reduce_params:       Option<Json>,
    lang:                Option<&'a str>,
    init_script_file:    Option<&'a str>,
    init_script_id:      Option<&'a str>,
    map_script_file:     Option<&'a str>,
    map_script_id:       Option<&'a str>,
    combine_script_file: Option<&'a str>,
    combine_script_id:   Option<&'a str>,
    reduce_script_file:  Option<&'a str>,
    reduce_script_id:    Option<&'a str>
}

impl<'a> ScriptedMetric<'a> {
    pub fn new(map_script: &'a str) -> ScriptedMetric<'a> {
        ScriptedMetric {
            init_script:         None,
            map_script:          map_script,
            combine_script:      None,
            reduce_script:       None,
            params:              None,
            reduce_params:       None,
            lang:                None,
            init_script_file:    None,
            init_script_id:      None,
            map_script_file:     None,
            map_script_id:       None,
            combine_script_file: None,
            combine_script_id:   None,
            reduce_script_file:  None,
            reduce_script_id:    None
        }
    }

    add_field!(with_init_script, init_script, &'a str);
    add_field!(with_combine_script, combine_script, &'a str);
    add_field!(with_reduce_script, reduce_script, &'a str);
    add_field!(with_params, params, Json);
    add_field!(with_reduce_params, reduce_params, Json);
    add_field!(with_lang, lang, &'a str);
    add_field!(with_init_script_file, init_script_file, &'a str);
    add_field!(with_init_script_id, init_script_id, &'a str);
    add_field!(with_map_script_file, map_script_file, &'a str);
    add_field!(with_map_script_id, map_script_id, &'a str);
    add_field!(with_combine_script_file, combine_script_file, &'a str);
    add_field!(with_combine_script_id, combine_script_id, &'a str);
    add_field!(with_reduce_script_file, reduce_script_file, &'a str);
    add_field!(with_reduce_script_id, reduce_script_id, &'a str);
}

impl<'a> ToJson for ScriptedMetric<'a> {
    fn to_json(&self) -> Json {
        let mut d = BTreeMap::new();
        d.insert("map_script".to_owned(), self.map_script.to_json());
        optional_add!(self, d, init_script);
        optional_add!(self, d, combine_script);
        optional_add!(self, d, reduce_script);
        optional_add!(self, d, params);
        optional_add!(self, d, reduce_params);
        optional_add!(self, d, lang);
        optional_add!(self, d, init_script_file);
        optional_add!(self, d, init_script_id);
        optional_add!(self, d, map_script_file);
        optional_add!(self, d, map_script_id);
        optional_add!(self, d, combine_script_file);
        optional_add!(self, d, combine_script_id);
        optional_add!(self, d, reduce_script_file);
        optional_add!(self, d, reduce_script_id);
        Json::Object(d)
    }
}

/// Individual aggregations and their options
#[derive(Debug)]
pub enum MetricsAggregation<'a> {
    Min(Min<'a>),
    // Max(Max<'a>),
    // Sum(Sum<'a>),
    // Avg(Avg<'a>),
    // Stats(Stats<'a>),
    // ExtendedStats(ExtendedStats<'a>),
    // ValueCount(ValueCount<'a>),
    // Percentiles(Percentiles<'a>),
    // PercentileRanks(PercentileRanks<'a>),
    // Cardinality(Cardinality<'a>),
    // GeoBounds(GeoBounds<'a>),
    // ScriptedMetric(ScriptedMetric<'a>)
}

impl<'a> MetricsAggregation<'a> {
    pub fn to_map(&self) -> BTreeMap<&'static str, Value> {
        use self::MetricsAggregation::*;
        let mut b = BTreeMap::new();
        let (key, value) = match self {
            &Min(ref min) => ("min", to_value(min))
        };
        println!("Self: {:?}", &self);
        println!("Value: {:?}", &value);
        b.insert(key, value);
        b
    }
}

// TODO deprecated
// impl<'a> ToJson for MetricsAggregation<'a> {
//     fn to_json(&self) -> Json {
//         let mut d = BTreeMap::new();
//         match self {
//             &MetricsAggregation::Min(ref min_agg) => {
//                 d.insert("min".to_owned(), min_agg.to_json());
//             },
//             &MetricsAggregation::Max(ref max_agg) => {
//                 d.insert("max".to_owned(), max_agg.to_json());
//             },
//             &MetricsAggregation::Sum(ref sum_agg) => {
//                 d.insert("sum".to_owned(), sum_agg.to_json());
//             },
//             &MetricsAggregation::Avg(ref avg_agg) => {
//                 d.insert("avg".to_owned(), avg_agg.to_json());
//             },
//             &MetricsAggregation::Stats(ref stats_agg) => {
//                 d.insert("stats".to_owned(), stats_agg.to_json());
//             },
//             &MetricsAggregation::ExtendedStats(ref ext_stat_agg) => {
//                 d.insert("extended_stats".to_owned(), ext_stat_agg.to_json());
//             },
//             &MetricsAggregation::ValueCount(ref vc_agg) => {
//                 d.insert("value_count".to_owned(), vc_agg.to_json());
//             },
//             &MetricsAggregation::Percentiles(ref pc_agg) => {
//                 d.insert("percentiles".to_owned(), pc_agg.to_json());
//             },
//             &MetricsAggregation::PercentileRanks(ref pr_agg) => {
//                 d.insert("percentile_ranks".to_owned(), pr_agg.to_json());
//             },
//             &MetricsAggregation::Cardinality(ref card_agg) => {
//                 d.insert("cardinality".to_owned(), card_agg.to_json());
//             },
//             &MetricsAggregation::GeoBounds(ref gb_agg) => {
//                 d.insert("geo_bounds".to_owned(), gb_agg.to_json());
//             },
//             &MetricsAggregation::ScriptedMetric(ref sm_agg) => {
//                 d.insert("scripted_metric".to_owned(), sm_agg.to_json());
//             }
//         }
//         Json::Object(d)
//     }
// }

#[cfg(test)]
pub mod tests {
    use serde_json;

    use super::super::Aggregations;
    use super::Min;

    #[test]
    fn test_min_aggregation() {
        let aggs:Aggregations = ("min_test", Min::field("blah")).into();

        assert_eq!("{\"min_test\":{\"min\":{\"field\":\"blah\"}}}",
                   serde_json::to_string(&aggs).unwrap());
    }
}
