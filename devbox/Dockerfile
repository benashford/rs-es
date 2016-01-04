FROM devbox-rust-beta

RUN apt-get update

# Compilation dependencies
RUN apt-get -y install libssl-dev

# Tool dependencies
RUN apt-get -y install ruby

RUN echo "export ES_HOST=es" > /home/ben/.profiles/es.sh
