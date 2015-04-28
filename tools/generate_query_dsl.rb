#!/usr/bin/env ruby

require 'erb'

E = Struct.new(:name, :json_name)

F = Struct.new(:name, :type, :optional)

class F
  def json_name
    if name == 'match_type'
      'type'
    else
      name
    end
  end

  def with
    "with_#{json_name}"
  end
end

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

          impl Query {
              <% fields.each do |field| %>
                  pub fn build_<%= field.json_name %>(
                     <% sfs = get_struct_fields(field.name).reject(&:optional); sfs.each do |sf| %>
                         <%= sf.name %>: <%= sf.type %><% if !last(sfs, sf) %>,<% end %>
                     <% end %>) -> <%= field.name %>Query {
                     <% if get_struct_fields(field.name).empty? %>
                         <%= field.name %>Query
                     <% else %>
                         <%= field.name %>Query {
                             <% sfs = get_struct_fields(field.name); sfs.each do |sf| %>
                                 <%= sf.name %>: <% if sf.optional %>
                                                     None
                                                 <% else %>
                                                     <%= sf.name %>
                                                 <% end %><% if !last(sfs, sf) %>,<% end %>
                             <% end %>
                          }
                      <% end %>
                  }
              <% end %>
          }

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
      {'MatchAllQuery'  => [],
       'MatchQuery'     => [
         f('field', 'String'),
         f('query', 'Json'),
         f('match_type', 'MatchType', true),
         f('cutoff_frequency', 'f64', true),
         f('lenient', 'bool', true)
       ].concat(common_match_options),
       'MultiMatchQuery' => [
         f('fields', 'Vec<String>'),
         f('query', 'Json'),
         f('use_dis_max', 'bool', true),
         f('match_type', 'MatchQueryType', true)
       ].concat(common_match_options)
      }
    end

    def get_struct_fields(struct_name)
      structs["#{struct_name}Query"]
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
                  with!(<%= op_f.with %>, <%= op_f.name %>, <%= op_f.type %>);
              <% end %>

              fn add_optionals(&self, m: &mut BTreeMap<String, Json>) {
                  <% fields.select(&:optional).each do |op_f| %>
                      optional_add!(m, self.<%= op_f.name %>, "<%= op_f.json_name %>");
                  <% end %>
              }

              pub fn build(&self) -> <%= enum_type %> {
                  <%= enum_name %>((*self).clone())
              }
          }
        END
        m
      end
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
