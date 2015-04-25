// Query DSL

use std::collections::BTreeMap;

use rustc_serialize::json::{Json, ToJson};

pub enum Query {
    MatchAll,
    Match(MatchQuery),
    MultiMatch(MultiMatchQuery)
}

use self::Query::{Match,
                  MatchAll,
                  MultiMatch};

impl ToJson for Query {
    fn to_json(&self) -> Json {
        let mut d = BTreeMap::<String, Json>::new();
        match self {
            &MatchAll          => {
                d.insert("match_all".to_string(), Json::Object(BTreeMap::new()));
            },
            &Match(ref q)      => { d.insert("match".to_string(), q.to_json()); },
            &MultiMatch(ref q) => { d.insert("multi_match".to_string(), q.to_json()); }
        }
        Json::Object(d)
    }
}

impl Query {
    pub fn build_match(field: String, query: Json) -> MatchQuery {
        MatchQuery {
            field:            field,
            query:            query,
            match_type:       None,
            cutoff_frequency: None,
            lenient:          None,
            match_options:    CommonMatchOptions::new()
        }
    }

    pub fn build_multi_match(fields: Vec<String>, query: Json) -> MultiMatchQuery {
        MultiMatchQuery {
            fields:        fields,
            query:         query,
            use_dis_max:   None,
            match_type:    None,
            match_options: CommonMatchOptions::new()
        }
    }
}

macro_rules! with {
    ($funcn:ident, $sn:ident, $t:ident) => {
        pub fn $funcn<'a>(&'a mut self, value: $t) -> &'a mut Self {
            self.$sn = Some(value);
            self
        }
    }
}

macro_rules! optional_add {
    ($map:ident, $sn:expr, $field:expr) => {
        match $sn {
            Some(ref value) => { $map.insert($field.to_string(), value.to_json()); }
            _               => ()
        }
    }
}

// Match queries

#[derive(Clone)]
pub enum ZeroTermsQuery {
    None,
    All
}

impl ToJson for ZeroTermsQuery {
    fn to_json(&self) -> Json {
        match self {
            &ZeroTermsQuery::None => "none".to_json(),
            &ZeroTermsQuery::All  => "all".to_json()
        }
    }
}

#[derive(Clone)]
pub enum Fuzziness {
    Auto,
    LevenshteinDistance(i64),
    Proportionate(f64)
}

impl ToJson for Fuzziness {
    fn to_json(&self) -> Json {
        use self::Fuzziness::{Auto, LevenshteinDistance, Proportionate};
        match self {
            &Auto                      => "auto".to_json(),
            &LevenshteinDistance(dist) => dist.to_json(),
            &Proportionate(prop)       => prop.to_json()
        }
    }
}

#[derive(Clone)]
struct CommonMatchOptions {
    analyzer:             Option<String>,
    boost:                Option<f64>,
    operator:             Option<String>,
    minimum_should_match: Option<i64>,
    fuzziness:            Option<Fuzziness>,
    prefix_length:        Option<i64>,
    max_expansions:       Option<i64>,
    rewrite:              Option<String>,
    zero_terms_query:     Option<ZeroTermsQuery>
}

impl CommonMatchOptions {
    fn new() -> CommonMatchOptions {
        CommonMatchOptions {
            analyzer:             None,
            boost:                None,
            operator:             None,
            minimum_should_match: None,
            fuzziness:            None,
            prefix_length:        None,
            max_expansions:       None,
            rewrite:              None,
            zero_terms_query:     None
        }
    }

    fn export_json(&self, objects: &mut BTreeMap<String, Json>) {
        optional_add!(objects, self.analyzer, "analyzer");
        optional_add!(objects, self.boost, "boost");
        optional_add!(objects, self.operator, "operator");
        optional_add!(objects, self.minimum_should_match, "minimum_should_match");
        optional_add!(objects, self.fuzziness, "fuzziness");
        optional_add!(objects, self.prefix_length, "prefix_length");
        optional_add!(objects, self.max_expansions, "max_expansions");
        optional_add!(objects, self.rewrite, "rewrite");
        optional_add!(objects, self.zero_terms_query, "zero_terms_query");
    }
}

trait ExtendsCommonMatch {
    fn common_options<'a>(&'a mut self) -> &'a mut CommonMatchOptions;

    fn with_analyzer<'a>(&'a mut self, value: String) -> &'a mut Self {
        self.common_options().analyzer = Some(value);
        self
    }

    fn with_boost<'a>(&'a mut self, value: f64) -> &'a mut Self {
        self.common_options().boost = Some(value);
        self
    }

    fn with_operator<'a>(&'a mut self, value: String) -> &'a mut Self {
        self.common_options().operator = Some(value);
        self
    }

    fn with_minimum_should_match<'a>(&'a mut self, value: i64) -> &'a mut Self {
        self.common_options().minimum_should_match = Some(value);
        self
    }

    fn with_fuzziness<'a>(&'a mut self, value: Fuzziness) -> &'a mut Self {
        self.common_options().fuzziness = Some(value);
        self
    }

    fn with_prefix_length<'a>(&'a mut self, value: i64) -> &'a mut Self {
        self.common_options().prefix_length = Some(value);
        self
    }

    fn with_max_expansions<'a>(&'a mut self, value: i64) -> &'a mut Self {
        self.common_options().max_expansions = Some(value);
        self
    }

    fn with_rewrite<'a>(&'a mut self, value: String) -> &'a mut Self {
        self.common_options().rewrite = Some(value);
        self
    }

    fn zero_terms_query<'a>(&'a mut self, value: ZeroTermsQuery) -> &'a mut Self {
        self.common_options().zero_terms_query = Some(value);
        self
    }
}

#[derive(Clone)]
pub enum MatchType {
    Phrase,
    PhrasePrefix
}

impl ToJson for MatchType {
    fn to_json(&self) -> Json {
        use self::MatchType::{Phrase, PhrasePrefix};
        match self {
            &Phrase =>       "phrase".to_json(),
            &PhrasePrefix => "phrase_prefix".to_json()
        }
    }
}

#[derive(Clone)]
pub struct MatchQuery {
    field:            String,
    query:            Json,
    match_type:       Option<MatchType>,
    cutoff_frequency: Option<f64>,
    lenient:          Option<bool>,
    match_options:    CommonMatchOptions
}

impl MatchQuery {
    with!(with_type, match_type, MatchType);
    with!(with_cutoff_frequency, cutoff_frequency, f64);
    with!(with_lenient, lenient, bool);

    pub fn build(&self) -> Query {
        Match((*self).clone())
    }
}

impl ExtendsCommonMatch for MatchQuery {
    fn common_options<'a>(&'a mut self) -> &'a mut CommonMatchOptions {
        &mut self.match_options
    }
}

impl ToJson for MatchQuery {
    fn to_json(&self) -> Json {
        let mut inner = BTreeMap::new();
        inner.insert("query".to_string(), self.query.clone());
        optional_add!(inner, self.match_type, "type");
        optional_add!(inner, self.cutoff_frequency, "cutoff_frequency");
        optional_add!(inner, self.lenient, "lenient");

        self.match_options.export_json(&mut inner);

        let mut d = BTreeMap::new();
        d.insert(self.field.clone(), Json::Object(inner));

        Json::Object(d)
    }
}

#[derive(Clone)]
pub enum MatchQueryType {
    BestFields,
    MostFields,
    CrossFields,
    Phrase,
    PhrasePrefix
}

impl ToJson for MatchQueryType {
    fn to_json(&self) -> Json {
        use self::MatchQueryType::{BestFields,
                                   MostFields,
                                   CrossFields,
                                   Phrase,
                                   PhrasePrefix};
        match self {
            &BestFields   => "best_fields".to_json(),
            &MostFields   => "most_fields".to_json(),
            &CrossFields  => "cross_fields".to_json(),
            &Phrase       => "phrase".to_json(),
            &PhrasePrefix => "phrase_prefix".to_json()
        }
    }
}

#[derive(Clone)]
pub struct MultiMatchQuery {
    fields:               Vec<String>,
    query:                Json,
    use_dis_max:          Option<bool>,
    match_type:           Option<MatchQueryType>,
    match_options:        CommonMatchOptions
}

impl MultiMatchQuery {
    with!(with_use_dis_max, use_dis_max, bool);
    with!(with_type, match_type, MatchQueryType);

    pub fn build(&self) -> Query {
        MultiMatch((*self).clone())
    }
}

impl ExtendsCommonMatch for MultiMatchQuery {
    fn common_options<'a>(&'a mut self) -> &'a mut CommonMatchOptions {
        &mut self.match_options
    }
}

impl ToJson for MultiMatchQuery {
    fn to_json(&self) -> Json {
        let mut d = BTreeMap::new();
        d.insert("query".to_string(), self.query.clone());
        d.insert("fields".to_string(), self.fields.to_json());
        optional_add!(d, self.use_dis_max, "use_dis_max");
        optional_add!(d, self.match_type, "type");

        self.match_options.export_json(&mut d);

        Json::Object(d)
    }
}
