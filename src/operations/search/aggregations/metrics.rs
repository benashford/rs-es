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

use std::collections::HashMap;

use serde::ser::{Serialize, Serializer, SerializeMap};
use serde_json::{from_value, Value};

use ::error::EsError;
use ::json::{MergeSerialize, NoOuter, serialize_map_optional_kv, ShouldSkip};
use ::units::{GeoBox, JsonVal};

use super::{Aggregation, AggregationResult};
use super::common::{Agg, Script};

macro_rules! metrics_agg {
    ($b:ident) => {
        agg!($b);

        impl<'a> From<$b<'a>> for Aggregation<'a> {
            fn from(from: $b<'a>) -> Aggregation<'a> {
                Aggregation::Metrics(MetricsAggregation::$b(from))
            }
        }
    }
}

/// Min aggregation
#[derive(Debug)]
pub struct Min<'a>(Agg<'a, NoOuter>);
metrics_agg!(Min);

#[derive(Debug)]
pub struct Max<'a>(Agg<'a, NoOuter>);
metrics_agg!(Max);

/// Sum aggregation
#[derive(Debug)]
pub struct Sum<'a>(Agg<'a, NoOuter>);
metrics_agg!(Sum);

/// Avg aggregation
#[derive(Debug)]
pub struct Avg<'a>(Agg<'a, NoOuter>);
metrics_agg!(Avg);

/// Stats aggregation
#[derive(Debug)]
pub struct Stats<'a>(Agg<'a, NoOuter>);
metrics_agg!(Stats);

/// Extended stats aggregation
#[derive(Debug)]
pub struct ExtendedStats<'a>(Agg<'a, NoOuter>);
metrics_agg!(ExtendedStats);

/// Value count aggregation
#[derive(Debug)]
pub struct ValueCount<'a>(Agg<'a, NoOuter>);
metrics_agg!(ValueCount);

/// Percentiles aggregation, see: https://www.elastic.co/guide/en/elasticsearch/reference/current/search-aggregations-metrics-percentile-aggregation.html
///
/// # Examples
///
/// ```
/// use rs_es::operations::search::aggregations::metrics::Percentiles;
///
/// let p1 = Percentiles::field("field_name").with_compression(100u64);
/// let p2 = Percentiles::field("field_name").with_percents(vec![10.0, 20.0]);
/// ```
#[derive(Debug)]
pub struct Percentiles<'a>(Agg<'a, PercentilesExtra>);
metrics_agg!(Percentiles);

#[derive(Debug, Default)]
pub struct PercentilesExtra {
    percents:    Option<Vec<f64>>,
    compression: Option<u64>
}

impl<'a> Percentiles<'a> {
    add_extra_option!(with_percents, percents, Vec<f64>);
    add_extra_option!(with_compression, compression, u64);
}

impl MergeSerialize for PercentilesExtra {
    fn merge_serialize<S>(&self,
                          serializer: &mut S) -> Result<(), S::Error>
        where S: SerializeMap {

        try!(serialize_map_optional_kv(serializer, "percents", &self.percents));
        serialize_map_optional_kv(serializer, "compression", &self.compression)
    }
}

/// Percentile Ranks aggregation
#[derive(Debug)]
pub struct PercentileRanks<'a>(Agg<'a, PercentileRanksExtra>);
metrics_agg!(PercentileRanks);

#[derive(Debug, Default)]
pub struct PercentileRanksExtra {
    values: Vec<f64>
}

impl<'a> PercentileRanks<'a> {
    pub fn with_values<A>(mut self, values: A) -> Self
        where A: Into<Vec<f64>> {

        self.0.extra.values = values.into();
        self
    }
}

impl MergeSerialize for PercentileRanksExtra {
    fn merge_serialize<S>(&self,
                          serializer: &mut S) -> Result<(), S::Error>
        where S: SerializeMap {
        serializer.serialize_entry("values", &self.values)
    }
}

/// Cardinality aggregation
#[derive(Debug)]
pub struct Cardinality<'a>(Agg<'a, CardinalityExtra>);
metrics_agg!(Cardinality);

#[derive(Debug, Default)]
pub struct CardinalityExtra {
    precision_threshold: Option<u64>,
    rehash:              Option<bool>
}

impl<'a> Cardinality<'a> {
    add_extra_option!(with_precision_threshold, precision_threshold, u64);
    add_extra_option!(with_rehash, rehash, bool);
}

impl MergeSerialize for CardinalityExtra {
    fn merge_serialize<S>(&self,
                          serializer: &mut S) -> Result<(), S::Error>
        where S: SerializeMap {

        try!(serialize_map_optional_kv(serializer,
                                       "precision_threshold",
                                       &self.precision_threshold));
        serialize_map_optional_kv(serializer, "rehash", &self.rehash)
    }
}

/// Geo Bounds aggregation
#[derive(Debug, Default, Serialize)]
pub struct GeoBounds<'a> {
    field:          &'a str,
    #[serde(skip_serializing_if="ShouldSkip::should_skip")]
    wrap_longitude: Option<bool>
}

impl<'a> GeoBounds<'a> {
    pub fn new(field: &'a str) -> Self {
        GeoBounds {
            field: field,
            ..Default::default()
        }
    }

    add_field!(with_wrap_longitude, wrap_longitude, bool);
}

/// Scripted method aggregation
#[derive(Debug, Default, Serialize)]
pub struct ScriptedMetric<'a> {
    #[serde(skip_serializing_if="ShouldSkip::should_skip")]
    init_script:         Option<&'a str>,
    map_script:          &'a str,
    #[serde(skip_serializing_if="ShouldSkip::should_skip")]
    combine_script:      Option<&'a str>,
    #[serde(skip_serializing_if="ShouldSkip::should_skip")]
    reduce_script:       Option<&'a str>,
    #[serde(skip_serializing_if="ShouldSkip::should_skip")]
    params:              Option<Value>, // TODO - should this be generified?
    #[serde(skip_serializing_if="ShouldSkip::should_skip")]
    reduce_params:       Option<Value>, // TODO - should this be generified?
    #[serde(skip_serializing_if="ShouldSkip::should_skip")]
    lang:                Option<&'a str>,
    #[serde(skip_serializing_if="ShouldSkip::should_skip")]
    init_script_file:    Option<&'a str>,
    #[serde(skip_serializing_if="ShouldSkip::should_skip")]
    init_script_id:      Option<&'a str>,
    #[serde(skip_serializing_if="ShouldSkip::should_skip")]
    map_script_file:     Option<&'a str>,
    #[serde(skip_serializing_if="ShouldSkip::should_skip")]
    map_script_id:       Option<&'a str>,
    #[serde(skip_serializing_if="ShouldSkip::should_skip")]
    combine_script_file: Option<&'a str>,
    #[serde(skip_serializing_if="ShouldSkip::should_skip")]
    combine_script_id:   Option<&'a str>,
    #[serde(skip_serializing_if="ShouldSkip::should_skip")]
    reduce_script_file:  Option<&'a str>,
    #[serde(skip_serializing_if="ShouldSkip::should_skip")]
    reduce_script_id:    Option<&'a str>
}

impl<'a> ScriptedMetric<'a> {
    pub fn new(map_script: &'a str) -> ScriptedMetric<'a> {
        ScriptedMetric {
            map_script: map_script,
            ..Default::default()
        }
    }

    add_field!(with_init_script, init_script, &'a str);
    add_field!(with_combine_script, combine_script, &'a str);
    add_field!(with_reduce_script, reduce_script, &'a str);
    add_field!(with_params, params, Value);
    add_field!(with_reduce_params, reduce_params, Value);
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

/// Individual aggregations and their options
#[derive(Debug)]
pub enum MetricsAggregation<'a> {
    Min(Min<'a>),
    Max(Max<'a>),
    Sum(Sum<'a>),
    Avg(Avg<'a>),
    Stats(Stats<'a>),
    ExtendedStats(ExtendedStats<'a>),
    ValueCount(ValueCount<'a>),
    Percentiles(Percentiles<'a>),
    PercentileRanks(PercentileRanks<'a>),
    Cardinality(Cardinality<'a>),
    GeoBounds(GeoBounds<'a>),
    ScriptedMetric(ScriptedMetric<'a>)
}

impl<'a> MetricsAggregation<'a> {
    pub fn details(&self) -> &'static str {
        use self::MetricsAggregation::*;
        match self {
            &Min(_) => "min",
            &Max(_) => "max",
            &Sum(_) => "sum",
            &Avg(_) => "avg",
            &Stats(_) => "stats",
            &ExtendedStats(_) => "extended_stats",
            &ValueCount(_) => "value_count",
            &Percentiles(_) => "percentiles",
            &PercentileRanks(_) => "percentile_ranks",
            &Cardinality(_) => "cardinality",
            &GeoBounds(_) => "geo_bounds",
            &ScriptedMetric(_) => "scripted_metric"
        }
    }
}

impl<'a> Serialize for MetricsAggregation<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where S: Serializer {
        use self::MetricsAggregation::*;
        match self {
            &Min(ref min) => min.serialize(serializer),
            &Max(ref max) => max.serialize(serializer),
            &Sum(ref sum) => sum.serialize(serializer),
            &Avg(ref avg) => avg.serialize(serializer),
            &Stats(ref stats) => stats.serialize(serializer),
            &ExtendedStats(ref extended_stats) => extended_stats.serialize(serializer),
            &ValueCount(ref value_count) => value_count.serialize(serializer),
            &Percentiles(ref percentiles) => percentiles.serialize(serializer),
            &PercentileRanks(ref percentile_ranks) => percentile_ranks.serialize(serializer),
            &Cardinality(ref cardinality) => cardinality.serialize(serializer),
            &GeoBounds(ref geo_bounds) => geo_bounds.serialize(serializer),
            &ScriptedMetric(ref scripted_metric) => scripted_metric.serialize(serializer)
        }
    }
}

// results

#[derive(Debug)]
pub enum MetricsAggregationResult {
    Min(MinResult),
    Max(MaxResult),
    Sum(SumResult),
    Avg(AvgResult),
    Stats(StatsResult),
    ExtendedStats(ExtendedStatsResult),
    ValueCount(ValueCountResult),
    Percentiles(PercentilesResult),
    PercentileRanks(PercentileRanksResult),
    Cardinality(CardinalityResult),
    GeoBounds(GeoBoundsResult),
    ScriptedMetric(ScriptedMetricResult)
}

impl MetricsAggregationResult {
    pub fn from<'a>(ma: &MetricsAggregation<'a>, json: &Value) -> Result<Self, EsError> {
        use self::MetricsAggregation::*;
        // TODO - must be a more efficient way to do this
        let json = json.clone();
        Ok(match ma {
            &Min(_) => {
                MetricsAggregationResult::Min(try!(from_value(json)))
            },
            &Max(_) => {
                MetricsAggregationResult::Max(try!(from_value(json)))
            },
            &Sum(_) => {
                MetricsAggregationResult::Sum(try!(from_value(json)))
            },
            &Avg(_) => {
                MetricsAggregationResult::Avg(try!(from_value(json)))
            },
            &Stats(_) => {
                MetricsAggregationResult::Stats(try!(from_value(json)))
            },
            &ExtendedStats(_) => {
                MetricsAggregationResult::ExtendedStats(try!(from_value(json)))
            },
            &ValueCount(_) => {
                MetricsAggregationResult::ValueCount(try!(from_value(json)))
            }
            &Percentiles(_) => {
                MetricsAggregationResult::Percentiles(try!(from_value(json)))
            },
            &PercentileRanks(_) => {
                MetricsAggregationResult::PercentileRanks(try!(from_value(json)))
            },
            &Cardinality(_) => {
                MetricsAggregationResult::Cardinality(try!(from_value(json)))
            },
            &GeoBounds(_) => {
                MetricsAggregationResult::GeoBounds(try!(from_value(json)))
            },
            &ScriptedMetric(_) => {
                MetricsAggregationResult::ScriptedMetric(try!(from_value(json)))
            }
        })
    }
}

macro_rules! metrics_agg_as {
    ($n:ident,$t:ident,$rt:ty) => {
        agg_as!($n,Metrics,MetricsAggregationResult,$t,$rt);
    }
}

impl AggregationResult {
    metrics_agg_as!(as_min, Min, MinResult);
    metrics_agg_as!(as_max, Max, MaxResult);
    metrics_agg_as!(as_sum, Sum, SumResult);
    metrics_agg_as!(as_avg, Avg, AvgResult);
    metrics_agg_as!(as_stats, Stats, StatsResult);
    metrics_agg_as!(as_extended_stats, ExtendedStats, ExtendedStatsResult);
    metrics_agg_as!(as_value_count, ValueCount, ValueCountResult);
    metrics_agg_as!(as_percentiles, Percentiles, PercentilesResult);
    metrics_agg_as!(as_percentile_ranks, PercentileRanks, PercentileRanksResult);
    metrics_agg_as!(as_cardinality, Cardinality, CardinalityResult);
    metrics_agg_as!(as_geo_bounds, GeoBounds, GeoBoundsResult);
    metrics_agg_as!(as_scripted_metric, ScriptedMetric, ScriptedMetricResult);
}

// specific result objects

/// Min Result
#[derive(Debug, Deserialize)]
pub struct MinResult {
    pub value: JsonVal
}

#[derive(Debug, Deserialize)]
pub struct MaxResult {
    pub value: JsonVal
}

#[derive(Debug, Deserialize)]
pub struct SumResult {
    pub value: f64
}

#[derive(Debug, Deserialize)]
pub struct AvgResult {
    pub value: f64
}

#[derive(Debug, Deserialize)]
pub struct StatsResult {
    pub count: u64,
    pub min: f64,
    pub max: f64,
    pub avg: f64,
    pub sum: f64
}

/// Used by the `ExtendedStatsResult`
#[derive(Debug, Deserialize)]
pub struct Bounds {
    pub upper: f64,
    pub lower: f64
}

#[derive(Debug, Deserialize)]
pub struct ExtendedStatsResult {
    pub count: u64,
    pub min: f64,
    pub max: f64,
    pub avg: f64,
    pub sum: f64,
    pub sum_of_squares: f64,
    pub variance: f64,
    pub std_deviation: f64,
    pub std_deviation_bounds: Bounds
}

#[derive(Debug, Deserialize)]
pub struct ValueCountResult {
    pub value: u64
}

#[derive(Debug, Deserialize)]
pub struct PercentilesResult {
    pub values: HashMap<String, f64>
}

#[derive(Debug, Deserialize)]
pub struct PercentileRanksResult {
    pub values: HashMap<String, f64>
}

#[derive(Debug, Deserialize)]
pub struct CardinalityResult {
    pub value: u64
}

#[derive(Debug, Deserialize)]
pub struct GeoBoundsResult {
    pub bounds: GeoBox
}

#[derive(Debug, Deserialize)]
pub struct ScriptedMetricResult {
    pub value: JsonVal
}

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
