FROM rust:1.64 

RUN git clone https://github.com/MrDahaniel/flatshare.git

WORKDIR /flatshare/flat-back

RUN cargo install --path .

EXPOSE 8080

CMD ["flat-back"]

