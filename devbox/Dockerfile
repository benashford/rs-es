FROM devbox-rust-nightly

RUN apt-get update

# Compilation dependencies
RUN apt-get -y install libssl-dev

RUN echo "export ES_HOST=es" > /home/ben/.profiles/es.sh
