FROM devbox-rust

RUN rustup update && \
    echo "export ES_HOST=http://es:9200" > /home/ben/.profiles/es.sh

USER root

RUN apt-get update

# Compilation dependencies
RUN apt-get -y install libssl-dev pkg-config
