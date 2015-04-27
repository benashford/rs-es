#!/usr/bin/env ruby

require 'erb'

E = Struct.new(:name, :json_name)
F = Struct.new(:name, :type, :optional)

class ESDSLGen
  class << self
    def e(name, json_name)
      E.new(name, json_name)
    end

    def f(name, type, optional = false)
      F.new(name, type, optional)
    end

    def enums
      {'Query' => [e('MatchAll', 'match_all'),
                   e('Match', 'match'),
                   e('MultiMatch', 'multi_match')]}
    end

    def last(col, item)
      col.last == item
    end

    def generate_enums
      enums.reduce({}) do |m, (name, fields)|
        m[name] = ERB.new(<<-END).result(binding)
          pub enum <%= name %> {
             <% fields.each do |field| %>
               <%= field.name %>(<%= field.name %>Query)<% if !last(fields, field) %>,<% end %>
             <% end %>
          }

          use self::<%= name %>::{<%= fields.map(&:name).join(',') %>};

          impl ToJson for Query {
              fn to_json(&self) -> Json {
                  let mut d = BTreeMap::<String, Json>::new();
                  match self {
                      <% fields.each do |field| %>
                          &<%= field.name %>(ref q) => {
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
        f('minimum_should_match', 'i64', true),
        f('fuzziness', 'Fuzziness', true),
        f('prefix_length', 'i64', true),
        f('max_expansions', 'i64', true),
        f('rewrite', 'String', true),
        f('zero_terms_query', 'ZeroTermsQuery', true)
      ]
    end

    def structs
      {'MatchQuery' => [
         f('field', 'String'),
         f('query', 'Json'),
         f('match_type', 'MatchType', true),
         f('cutoff_frequency', 'f64', true),
         f('lenient', 'bool', true)
       ].concat(common_match_options)
      }
    end

    def generate_structs
      structs.reduce({}) do |m, (name, fields)|
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
        END
        m
      end
    end

    def generate
      {enums:   generate_enums,
       structs: generate_structs,}
    end
  end
end

puts ESDSLGen.generate
