FROM devbox-rust-nightly

RUN rustup update && \
    echo "export ES_HOST=es" > /home/ben/.profiles/es.sh

USER root

RUN apt-get update

# Compilation dependencies
RUN apt-get -y install libssl-dev
