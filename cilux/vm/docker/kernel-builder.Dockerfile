FROM --platform=linux/arm64 ubuntu:24.04

ENV DEBIAN_FRONTEND=noninteractive

RUN apt-get update && apt-get install -y \
    bc \
    bison \
    build-essential \
    ca-certificates \
    clang \
    cpio \
    curl \
    flex \
    git \
    libelf-dev \
    libssl-dev \
    lld \
    llvm \
    pahole \
    pkg-config \
    python3 \
    rsync \
    xz-utils \
 && rm -rf /var/lib/apt/lists/*

RUN curl https://sh.rustup.rs -sSf | sh -s -- -y --profile minimal
ENV PATH=/root/.cargo/bin:/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin

RUN rustup toolchain install stable --profile minimal \
 && rustup default stable \
 && rustup component add rust-src \
 && cargo install bindgen-cli

WORKDIR /workspace
