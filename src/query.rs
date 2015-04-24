// Query DSL

use std::collections::BTreeMap;

use rustc_serialize::json::{Json, ToJson};

pub enum Query {
    Match(MatchQuery),
    MatchAll
}

use self::Query::{Match,
                  MatchAll};

impl ToJson for Query {
    fn to_json(&self) -> Json {
        let mut d = BTreeMap::<String, Json>::new();
        match self {
            &MatchAll     => {
                d.insert("match_all".to_string(), Json::Object(BTreeMap::new()));
            },
            &Match(ref q) => { d.insert("match".to_string(), q.to_json()); },
        }
        Json::Object(d)
    }
}

impl Query {
    pub fn build_match(field: String, query: String) -> MatchQuery {
        MatchQuery {
            field:            field,
            query:            query,
            operator:         None,
            zero_terms_query: None,
            cutoff_frequency: None,
            lenient:          None
        }
    }
}

macro_rules! with {
    ($funcn:ident, $sn:ident, $t:ident, $rt:ident) => {
        pub fn $funcn<'a>(&'a mut self, value: $t) -> &'a mut $rt {
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

pub struct MatchQuery {
    field:            String,
    query:            String,
    operator:         Option<String>,
    zero_terms_query: Option<String>,
    cutoff_frequency: Option<f64>,
    lenient:          Option<bool>
}

impl MatchQuery {
    with!(with_operator, operator, String, MatchQuery);
    with!(with_zero_terms_query, zero_terms_query, String, MatchQuery);
    with!(with_cutoff_frequency, cutoff_frequency, f64, MatchQuery);
    with!(with_lenient, lenient, bool, MatchQuery);

    pub fn build(self) -> Query {
        Match(self)
    }
}

impl ToJson for MatchQuery {
    fn to_json(&self) -> Json {
        let mut inner = BTreeMap::new();
        inner.insert("query".to_string(), self.query.to_json());
        optional_add!(inner, self.operator, "operator");
        optional_add!(inner, self.zero_terms_query, "zero_terms_query");
        optional_add!(inner, self.cutoff_frequency, "cutoff_frequency");
        optional_add!(inner, self.lenient, "lenient");

        let mut d = BTreeMap::new();
        d.insert(self.field.clone(), Json::Object(inner));

        Json::Object(d)
    }
}
