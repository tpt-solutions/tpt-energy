# Container image for the `tpt-nrg` CLI.
#
# The binary is statically linked against musl so the runtime stage can be
# `FROM scratch`: no libc, no shell, no package manager, and nothing to patch.
# The image is what `release.yml`'s `docker` job pushes to GHCR on a tag.
#
#   docker build -t tpt-nrg .
#   docker run --rm -v "$PWD:/cases" tpt-nrg run --system /cases/case.m
#
# Build for a different target with:
#   docker build --build-arg TARGET=aarch64-unknown-linux-musl -t tpt-nrg .

# --- build ------------------------------------------------------------------
# The workspace's MSRV is 1.84, so any 1.x toolchain builds it; tracking the
# `1` tag keeps the image from bit-rotting on a pinned patch release.
FROM rust:1-alpine AS build

# `x86_64-unknown-linux-musl` keeps the default `docker build` producing a
# static binary; a multi-arch build passes its own `TARGET`.
ARG TARGET=x86_64-unknown-linux-musl

RUN apk add --no-cache musl-dev

WORKDIR /src

# Copy the manifests first so dependency compilation is cached independently of
# the source, then the sources.
COPY Cargo.toml Cargo.lock ./
COPY crates ./crates

RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/src/target \
    cargo build --release --locked --target "$TARGET" -p tpt-nrg-cli \
    && cp "/src/target/$TARGET/release/tpt-nrg" /out-tpt-nrg

# --- runtime ----------------------------------------------------------------
FROM scratch AS runtime

ARG TARGET=x86_64-unknown-linux-musl
LABEL org.opencontainers.image.title="tpt-nrg" \
      org.opencontainers.image.description="Power-systems analysis CLI from TPT Energy" \
      org.opencontainers.image.source="https://github.com/tpt-solutions/tpt-energy" \
      org.opencontainers.image.licenses="MIT OR Apache-2.0"

COPY --from=build /out-tpt-nrg /usr/local/bin/tpt-nrg

# `tpt-nrg` reads and writes files, so the working directory has to be
# bind-mounted by the caller; this is the default when none is given.
WORKDIR /cases

ENTRYPOINT ["/usr/local/bin/tpt-nrg"]
CMD ["--help"]
