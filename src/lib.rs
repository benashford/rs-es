#![crate_type = "lib"]
#![crate_name = "rs_es"]

#[macro_use] extern crate log;
extern crate hyper;

struct Client {
    http_client: hyper::Client
}

impl Client {
    fn new() -> Client {
        Client {
            http_client: hyper::Client::new()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Client;

    #[test]
    fn it_works() {
        let mut client = Client::new();

    }
}
