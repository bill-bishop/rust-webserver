FROM rust:1.21.0
WORKDIR /usr/src/rustserver
COPY . .
RUN cargo install
CMD ["rustserver"]
