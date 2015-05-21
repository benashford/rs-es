#!/usr/bin/env ruby
# coding: utf-8

# Copyright 2015 Ben Ashford
#
# Licensed under the Apache License, Version 2.0 (the "License");
# you may not use this file except in compliance with the License.
# You may obtain a copy of the License at
#
#     http://www.apache.org/licenses/LICENSE-2.0
#
# Unless required by applicable law or agreed to in writing, software
# distributed under the License is distributed on an "AS IS" BASIS,
# WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
# See the License for the specific language governing permissions and
# limitations under the License.

require 'erb'

# miscellaneous utilities

def snake_to_camel(field)
  field.split('_').map(&:capitalize).join
end

# Encapsulates details of an enum value
class EnumVal
  attr_accessor :name, :json_name

  def initialize(json_name)
    self.name = snake_to_camel(json_name)
    self.json_name = json_name
  end
end

# Encapsulates details of a field
class Field
  JSON_SUBS = {'match_type' => 'type',
               'doc_type'   => 'type',
               'span_match' => 'match'}

  # Name of the field
  attr_accessor :name

  # Type of the field in the struct
  attr_accessor :type

  # Type of the field in parameter form (e.g. type may be 'String', but
  # parameter will be 'Into<String>')
  attr_accessor :param_type

  # Optional field
  attr_accessor :optional

  def initialize(name, type, optional)
    self.name       = name
    self.type       = type
    self.param_type = "Into<#{type}>"
    self.optional   = optional
  end

  def json_name
    JSON_SUBS[name] || name
  end

  def with
    "with_#{json_name.gsub(/^_/, '')}"
  end
end

class ESDSLGen
  class << self
    def f(name, type, optional = false)
      Field.new(name, type, optional)
    end

    ENUM_NAMES = %w[match_all match multi_match bool boosting common constant_score
                    dis_max filtered fuzzy_like_this fuzzy_like_this_field function_score
                    fuzzy geo_shape has_child has_parent ids indices more_like_this nested
                    prefix query_string simple_query_string range regexp span_first
                    span_multi span_near span_not span_or span_term term terms wildcard]

    FUNCTION_NAMES = %w[script_score random_score weight]

    FILTER_NAMES = %w[and bool exists geo_bounding_box geo_distance geo_polygon geo_shape
                      geohash_cell has_child has_parent ids indices match_all missing
                      nested not or prefix query range regexp script term terms type]

    def enums
      {
        'Query'    => ENUM_NAMES.map {|n| EnumVal.new(n) },
        'Function' => FUNCTION_NAMES.map {|n| EnumVal.new(n) },
        'Filter'   => FILTER_NAMES.map {|n| EnumVal.new(n) }
      }
    end

    def last(col, item)
      col.last == item
    end

    def generate_enums
      enums.reduce({}) do |m, (name, fields)|
        m[name] = ERB.new(<<-END).result(binding)
          #[derive(Clone)]
          pub enum <%= name %> {
             <% fields.each do |field| %>
               <%= field.name %>(<%= field.name %><%= name %>)
               <% if !last(fields, field) %>,<% end %>
             <% end %>
          }

          impl <%= name %> {
              <% fields.each do |field| %>
                  pub fn build_<%= field.json_name %><%= generate_type_params(field.name, name) %>(
                     <% sfs = get_optional_struct_fields(field.name, name);
                        sfs.zip(ALPHABET).each do |(sf, letter)| %>
                         <%= sf.name %>: <%= letter %><% if !last(sfs, sf) %>,<% end %>
                     <% end %>) -> <%= field.name %><%= name %> {
                     <% if get_struct_fields(field.name, name).empty? %>
                         <%= field.name %><%= name %>
                     <% else %>
                         <%= field.name %><%= name %> {
                             <% sfs = get_struct_fields(field.name, name); sfs.each do |sf| %>
                                 <%= sf.name %>: <% if sf.optional %>
                                                     None
                                                 <% else %>
                                                     <%= sf.name %>.into()
                                                 <% end %><% if !last(sfs, sf) %>,<% end %>
                             <% end %>
                          }
                      <% end %>
                  }
              <% end %>
          }

          impl ToJson for <%= name %> {
              fn to_json(&self) -> Json {
                  let mut d = BTreeMap::<String, Json>::new();
                  match self {
                      <% fields.each do |field| %>
                          &<%= name %>::<%= field.name %>(ref q) => {
                              d.insert("<%= field.json_name %>".to_string(), q.to_json());
                          }<% if !last(fields, field) %>,<% end %>
                      <% end %>
                  }
                  Json::Object(d)
              }
          }
        END
        m
      end
    end

    def common_match_options
      [
        f('analyzer', 'String', true),
        f('boost', 'f64', true),
        f('operator', 'String', true),
        f('minimum_should_match', 'MinimumShouldMatch', true),
        f('fuzziness', 'Fuzziness', true),
        f('prefix_length', 'i64', true),
        f('max_expansions', 'i64', true),
        f('rewrite', 'String', true),
        f('zero_terms_query', 'ZeroTermsQuery', true)
      ]
    end

    def common_filter_options
      [
        f('_cache', 'bool', true),
        f('_cache_key', 'String', true),
        f('_name', 'String', true)
      ]
    end

    def structs
      query_structs = {'MatchAllQuery'  => [
                         f('boost', 'f64', true)
                       ],
                       'MatchQuery'     => [
                         f('field', 'String'),
                         f('query', 'JsonVal'),
                         f('match_type', 'MatchType', true),
                         f('cutoff_frequency', 'f64', true),
                         f('lenient', 'bool', true)
                       ].concat(common_match_options),
                       'MultiMatchQuery' => [
                         f('fields', 'Vec<String>'),
                         f('query', 'JsonVal'),
                         f('use_dis_max', 'bool', true),
                         f('match_type', 'MatchQueryType', true)
                       ].concat(common_match_options),
                       'BoolQuery' => [
                         f('must', 'Vec<Query>', true),
                         f('must_not', 'Vec<Query>', true),
                         f('should', 'Vec<Query>', true),
                         f('minimum_should_match', 'MinimumShouldMatch', true),
                         f('boost', 'f64', true)
                       ],
                       'BoostingQuery' => [
                         f('positive', 'Box<Query>', true),
                         f('negative', 'Box<Query>', true),
                         f('negative_boost', 'f64', true)
                       ],
                       'CommonQuery' => [
                         f('query', 'JsonVal'),
                         f('cutoff_frequency', 'f64', true),
                         f('low_freq_operator', 'String', true),
                         f('high_freq_operator', 'String', true),
                         f('minimum_should_match', 'MinimumShouldMatch', true),
                         f('boost', 'f64', true),
                         f('analyzer', 'String', true),
                         f('disable_coord', 'bool', true)
                       ],
                       'ConstantScoreQuery' => [
                         f('filter', 'Box<Filter>', true),
                         f('query', 'Box<Query>', true),
                         f('boost', 'f64', true)
                       ],
                       'DisMaxQuery' => [
                         f('tie_breaker', 'f64', true),
                         f('boost', 'f64', true),
                         f('queries', 'Vec<Query>')
                       ],
                       'FilteredQuery' => [
                         f('filter', 'Box<Filter>'),
                         f('query', 'Box<Query>', true),
                         f('strategy', 'Strategy', true)
                       ],
                       'FuzzyLikeThisQuery' => [
                         f('fields', 'Vec<String>', true),
                         f('like_text', 'String'),
                         f('ignore_tf', 'bool', true),
                         f('max_query_terms', 'i64', true),
                         f('fuzziness', 'Fuzziness', true),
                         f('prefix_length', 'i64', true),
                         f('boost', 'f64', true),
                         f('analyzer', 'String', true)
                       ],
                       'FuzzyLikeThisFieldQuery' => [
                         f('field', 'String'),
                         f('like_text', 'String'),
                         f('ignore_tf', 'bool', true),
                         f('max_query_terms', 'i64', true),
                         f('fuzziness', 'Fuzziness', true),
                         f('prefix_length', 'i64', true),
                         f('boost', 'f64', true),
                         f('analyzer', 'String', true)
                       ],
                       'FunctionScoreQuery' => [
                         f('query', 'Box<Query>', true),
                         f('filter', 'Box<Filter>', true),
                         f('boost', 'f64', true),
                         f('functions', 'Vec<Function>'),
                         f('max_boost', 'f64', true),
                         f('score_mode', 'ScoreMode', true),
                         f('boost_mode', 'BoostMode', true),
                         f('min_score', 'f64', true)
                       ],
                       'FuzzyQuery' => [
                         f('field', 'String'),
                         f('value', 'String'),
                         f('boost', 'f64', true),
                         f('fuzziness', 'Fuzziness', true),
                         f('prefix_length', 'i64', true),
                         f('max_expansions', 'i64', true)
                       ],
                       'GeoShapeQuery' => [
                         f('field', 'String'),
                         f('shape', 'Shape', true),
                         f('indexed_shape', 'IndexedShape', true)
                       ],
                       'HasChildQuery' => [
                         f('doc_type', 'String'),
                         f('query', 'Box<Query>'),
                         f('score_mode', 'ScoreMode', true),
                         f('min_children', 'i64', true),
                         f('max_children', 'i64', true)
                       ],
                       'HasParentQuery' => [
                         f('parent_type', 'String'),
                         f('query', 'Box<Query>'),
                         f('score_mode', 'ScoreMode', true)
                       ],
                       'IdsQuery' => [
                         f('doc_type', 'OneOrMany<String>', true),
                         f('values', 'Vec<String>')
                       ],
                       'IndicesQuery' => [
                         f('index', 'String', true),
                         f('indices', 'Vec<String>', true),
                         f('query', 'Box<Query>'),
                         f('no_match_query', 'Box<Query>', true)
                       ],
                       'MoreLikeThisQuery' => [
                         f('fields', 'Vec<String>', true),
                         f('like_text', 'String', true),
                         f('ids', 'Vec<String>', true),
                         f('docs', 'Vec<Doc>', true),
                         f('max_query_terms', 'i64', true),
                         f('min_term_freq', 'i64', true),
                         f('min_doc_freq', 'i64', true),
                         f('max_doc_freq', 'i64', true),
                         f('min_word_length', 'i64', true),
                         f('max_word_length', 'i64', true),
                         f('stop_words', 'Vec<String>', true),
                         f('analyzer', 'String', true),
                         f('minimum_should_match', 'MinimumShouldMatch', true),
                         f('boost_terms', 'f64', true),
                         f('include', 'bool', true),
                         f('boost', 'f64', true)
                       ],
                       'NestedQuery' => [
                         f('path', 'String'),
                         f('score_mode', 'ScoreMode', true),
                         f('query', 'Box<Query>')
                       ],
                       'PrefixQuery' => [
                         f('field', 'String'),
                         f('value', 'String'),
                         f('boost', 'f64', true),
                         f('rewrite', 'Rewrite', true)
                       ],
                       'QueryStringQuery' => [
                         f('query', 'String'),
                         f('default_field', 'String', true),
                         f('default_operator', 'String', true),
                         f('analyzer', 'String', true),
                         f('allow_leading_wildcard', 'bool', true),
                         f('lowercase_expanded_terms', 'bool', true),
                         f('enable_position_increments', 'bool', true),
                         f('fuzzy_max_expansions', 'i64', true),
                         f('fuzziness', 'Fuzziness', true),
                         f('fuzzy_prefix_length', 'i64', true),
                         f('phrase_slop', 'i64', true),
                         f('boost', 'f64', true),
                         f('analyze_wildcard', 'bool', true),
                         f('auto_generate_phrase_queries', 'bool', true),
                         f('max_determined_states', 'i64', true),
                         f('minimum_should_match', 'MinimumShouldMatch', true),
                         f('lenient', 'bool', true),
                         f('locale', 'String', true),
                         f('time_zone', 'String', true)
                       ],
                       'SimpleQueryStringQuery' => [
                         f('query', 'String'),
                         f('fields', 'Vec<String>', true),
                         f('default_operator', 'String', true),
                         f('analyzer', 'String', true),
                         f('flags', 'String', true),
                         f('lowercase_expanded_terms', 'bool', true),
                         f('locale', 'String', true),
                         f('lenient', 'bool', true),
                         f('minimum_should_match', 'MinimumShouldMatch', true)
                       ],
                       'RangeQuery' => [
                         f('field', 'String'),
                         f('gte', 'JsonVal', true),
                         f('gt', 'JsonVal', true),
                         f('lte', 'JsonVal', true),
                         f('lt', 'JsonVal', true),
                         f('boost', 'f64', true),
                         f('time_zone', 'String', true),
                         f('format', 'String', true)
                       ],
                       'RegexpQuery' => [
                         f('field', 'String'),
                         f('value', 'String'),
                         f('boost', 'f64', true),
                         f('flags', 'Flags', true),
                         f('max_determined_states', 'i64', true)
                       ],
                       'SpanFirstQuery' => [
                         f('span_match', 'Box<Query>'),
                         f('end', 'i64')
                       ],
                       'SpanMultiQuery' => [
                         f('span_match', 'Box<Query>')
                       ],
                       'SpanNearQuery' => [
                         f('clauses', 'Vec<Query>'),
                         f('slop', 'i64'),
                         f('in_order', 'bool', true),
                         f('collect_payloads', 'bool', true)
                       ],
                       'SpanNotQuery' => [
                         f('include', 'Box<Query>'),
                         f('exclude', 'Box<Query>'),
                         f('pre', 'i64', true),
                         f('post', 'i64', true),
                         f('dist', 'i64', true)
                       ],
                       'SpanOrQuery' => [
                         f('clauses', 'Vec<Query>')
                       ],
                       'SpanTermQuery' => [
                         f('field', 'String'),
                         f('value', 'JsonVal'),
                         f('boost', 'f64', true)
                       ],
                       'TermQuery' => [
                         f('field', 'String'),
                         f('value', 'JsonVal'),
                         f('boost', 'f64', true)
                       ],
                       'TermsQuery' => [
                         f('field', 'String'),
                         f('values', 'Vec<JsonVal>'),
                         f('minimum_should_match', 'MinimumShouldMatch', true)
                       ],
                       'WildcardQuery' => [
                         f('field', 'String'),
                         f('value', 'String'),
                         f('boost', 'f64', true)
                       ]
                      }

      function_structs = {'ScriptScoreFunction' => [
                            f('script', 'String'),
                            f('lang', 'String', true),
                            f('params', 'HashMap<String, JsonVal>', true)
                          ],
                          'WeightFunction' => [
                            f('weight', 'f64')
                          ],
                          'RandomScoreFunction' => [
                            f('seed', 'i64', true)
                          ]}

      filter_structs = {'AndFilter' => [
                          f('filters', 'Vec<Filter>', true),
                        ],
                        'BoolFilter' => [
                          f('must', 'Vec<Filter>', true),
                          f('must_not', 'Vec<Filter>', true),
                          f('should', 'Vec<Filter>', true)
                        ],
                        'ExistsFilter' => [
                          f('field', 'String')
                        ],
                        'GeoBoundingBoxFilter' => [
                          f('field', 'String'),
                          f('geo_box', 'GeoBox')
                        ],
                        'GeoDistanceFilter' => [
                          f('field', 'String'),
                          f('location', 'Location'),
                          f('distance', 'Distance'),
                          f('distance_type', 'DistanceType', true),
                          f('optimize_bbox', 'OptimizeBbox', true)
                        ],
                        'GeoPolygonFilter' => [
                          f('field', 'String'),
                          f('points', 'Vec<Location>')
                        ],
                        'GeoShapeFilter' => [
                          f('field', 'String'),
                          f('shape', 'Shape', true),
                          f('indexed_shape', 'IndexedShape', true)
                        ],
                        'GeohashCellFilter' => [
                          f('field', 'String'),
                          f('location', 'Location'),
                          f('precision', 'Precision', true),
                          f('neighbors', 'bool', true)
                        ],
                        'HasChildFilter' => [
                          f('doc_type', 'String'),
                          f('query', 'Box<Query>', true),
                          f('filter', 'Box<Filter>', true),
                          f('min_children', 'i64', true),
                          f('max_children', 'i64', true)
                        ],
                        'HasParentFilter' => [
                          f('parent_type', 'String'),
                          f('query', 'Box<Query>', true),
                          f('filter', 'Box<Filter>', true)
                        ],
                        'IdsFilter' => [
                          f('doc_type', 'OneOrMany<String>', true),
                          f('values', 'Vec<String>')
                        ],
                        'IndicesFilter' => [
                          f('index', 'String', true),
                          f('indices', 'Vec<String>', true),
                          f('filter', 'Box<Filter>', true),
                          f('no_match_filter', 'NoMatchFilter', true)
                        ],
                        'MatchAllFilter' => [],
                        'MissingFilter' => [
                          f('field', 'String'),
                          f('existence', 'bool', true),
                          f('null_value', 'bool', true)
                        ],
                        'NestedFilter' => [
                          f('path', 'String'),
                          f('filter', 'Box<Filter>'),
                          f('score_mode', 'ScoreMode', true),
                          f('join', 'bool', true)
                        ],
                        'NotFilter' => [
                          f('filter', 'Box<Filter>')
                        ],
                        'OrFilter' => [
                          f('filters', 'Vec<Filter>')
                        ],
                        'PrefixFilter' => [
                          f('field', 'String'),
                          f('value', 'String')
                        ],
                        'QueryFilter' => [
                          f('query', 'Box<Query>')
                        ],
                        'RangeFilter' => [
                          f('field', 'String'),
                          f('gte', 'JsonVal', true),
                          f('gt', 'JsonVal', true),
                          f('lte', 'JsonVal', true),
                          f('lt', 'JsonVal', true),
                          f('boost', 'f64', true),
                          f('time_zone', 'String', true),
                          f('format', 'String', true)
                        ],
                        'RegexpFilter' => [
                          f('field', 'String'),
                          f('value', 'String'),
                          f('boost', 'f64', true),
                          f('flags', 'Flags', true),
                          f('max_determined_states', 'i64', true)
                        ],
                        'ScriptFilter' => [
                          f('script', 'String'),
                          f('params', 'BTreeMap<String, JsonVal>', true)
                        ],
                        'TermFilter' => [
                          f('field', 'String'),
                          f('value', 'JsonVal')
                        ],
                        'TermsFilter' => [
                          f('field', 'String'),
                          f('values', 'Vec<JsonVal>'),
                          f('execution', 'Execution', true),
                        ],
                        'TypeFilter' => [
                          f('value', 'String')
                        ]
                       }

      query_structs.tap do |all_structs|
        filter_structs.each do |name, fields|
          all_structs[name] = fields.concat(common_filter_options)
        end
        all_structs.merge!(function_structs)
      end
    end

    def get_struct_fields(struct_name, type)
      structs["#{struct_name}#{type}"]
    end

    def get_optional_struct_fields(struct_name, type)
      get_struct_fields(struct_name, type).reject(&:optional)
    end

    ALPHABET = ('A'..'Z').to_a

    def generate_type_params(struct_name, type)
      optional_fields = get_optional_struct_fields(struct_name, type)
      if optional_fields.count > 0
        params = optional_fields.zip(ALPHABET).map do |(field, letter)|
          "#{letter}: #{field.param_type}"
        end
        "<#{params.join(',')}>"
      else
        ''
      end
    end

    def generate_structs
      structs.reduce({}) do |m, (name, fields)|
        parts = name.split(/(?=[A-Z])/)
        enum_type = parts.pop
        enum_name = parts.join('')
        m[name] = ERB.new(<<-END).result(binding)
          #[derive(Clone)]
          pub struct <%= name %> {
              <% fields.each do |field| %>
                  <%= field.name %>: <% if field.optional %>
                                         Option<<%= field.type %>>
                                      <% else %>
                                         <%= field.type %>
                                      <% end %><% if !last(fields, field) %>,<% end %>
              <% end %>
          }

          impl <%= name %> {
              <% fields.select(&:optional).each do |op_f| %>
                  pub fn <%= op_f.with %><'a, T: <%= op_f.param_type %>>(&'a mut self, value: T) -> &'a mut Self {
                      self.<%= op_f.name %> = Some(value.into());
                      self
                  }
              <% end %>

              #[allow(dead_code, unused_variables)]
              fn add_optionals(&self, m: &mut BTreeMap<String, Json>) {
                  <% fields.select(&:optional).reject {|f| /^_/ =~ f.json_name }.each do |op_f| %>
                      optional_add!(m, self.<%= op_f.name %>, "<%= op_f.json_name %>");
                  <% end %>
              }

              #[allow(dead_code, unused_variables)]
              fn add_core_optionals(&self, m: &mut BTreeMap<String, Json>) {
                  <% fields.select(&:optional).select {|f| /^_/ =~ f.json_name }.each do |op_f| %>
                      optional_add!(m, self.<%= op_f.name %>, "<%= op_f.json_name %>");
                  <% end %>
              }

              pub fn build(&self) -> <%= enum_type %> {
                  <%= enum_type %>::<%= enum_name %>(self.clone())
              }
          }
        END
        m
      end
    end

    def to_json_impl(struct_name)
      fields = structs[struct_name]
      ERB.new(<<-END).result(binding)
        impl ToJson for <%= struct_name %> {
            fn to_json(&self) -> Json {
                let mut d = BTreeMap::new();
                <% fields.reject(&:optional).each do |field| %>
                  d.insert("<%= field.json_name %>".to_string(),
                           self.<%= field.name %>.to_json());
                <% end %>
                self.add_optionals(&mut d);
                self.add_core_optionals(&mut d);
                Json::Object(d)
            }
        }
      END
    end

    # Many queries/filters follow a pattern where the field is the key, and all
    # parameters belong to the inner object
    def to_json_inner_impl(struct_name)
      all_fields   = structs[struct_name].group_by {|f| f.name == 'field' }
      fields       = all_fields[false]
      field_fields = all_fields[true]
      raise "No field fields for #{struct_name}" if field_fields.nil?
      raise "Too many field fields" if field_fields.count > 1

      ERB.new(<<-END).result(binding)
        impl ToJson for <%= struct_name %> {
            fn to_json(&self) -> Json {
                let mut d = BTreeMap::new();
                let mut inner = BTreeMap::new();

                <% fields.reject(&:optional).each do |field| %>
                  inner.insert("<%= field.json_name %>".to_string(),
                               self.<%= field.name %>.to_json());
                <% end %>
                self.add_optionals(&mut inner);
                d.insert(self.field.clone(), Json::Object(inner));
                self.add_core_optionals(&mut d);
                Json::Object(d)
            }
        }
      END
    end

    def simple_value_enum(name, fields)
      ERB.new(<<-END).result(binding)
        #[derive(Clone)]
        pub enum <%= name %> {
            <% fields.each do |field| %>
                <%= snake_to_camel(field) %>
                <% if !last(fields, field) %>,<% end %>
            <% end %>
        }

        impl ToJson for <%= name %> {
            fn to_json(&self) -> Json {
                match self {
                    <% fields.each do |field| %>
                        &<%= name %>::<%= snake_to_camel(field) %>
                        => "<%= field %>".to_json()
                        <% if !last(fields, field) %>,<% end %>
                    <% end %>
                }
            }
        }
      END
    end

    def generate
      enums = generate_enums
      structs = generate_structs

      template = File.read('templates/query.rs.erb')
      result_file = ERB.new(template).result(binding)
      File.open('src/query.rs', 'w') do |file|
        file << result_file
      end
    end
  end
end

puts ESDSLGen.generate
