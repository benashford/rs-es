build: src/query.rs
	cargo build

src/query.rs: tools/generate_query_dsl.rb templates/query.rs.erb
	tools/generate_query_dsl.rb .

test: src/query.rs
	cargo test

clean:
	rm src/query.rs
	cargo clean
