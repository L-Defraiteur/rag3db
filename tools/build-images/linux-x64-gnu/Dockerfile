# Image de bâti de rag3weaver-backend pour Linux x86_64 (glibc).
#
# Base manylinux_2_28 (AlmaLinux 8, glibc 2.28) : la même que lucivy pour ses
# bindings Linux, pour que le binaire tourne sur toute distribution de 2019 ou
# plus récente. L'image ne contient que les outils ; le dépôt est monté sur
# /src et le cache de cargo sur /cache au moment du bâti (voir build.sh).
#
# Bâtir l'image :   docker build -t rag3weaver-build-linux-x64-gnu tools/build-images/linux-x64-gnu
# S'en servir :     tools/build-images/build.sh linux-x64-gnu
FROM quay.io/pypa/manylinux_2_28_x86_64

ARG RUST_VERSION=1.98.1

# gcc-toolset-14 est déjà dans le PATH de l'image. cmake et ninja viennent de
# pip (les paquets dnf d'AlmaLinux 8 sont plus vieux que ce que le moteur veut
# pour certaines options) ; git et python3 sont là.
RUN /opt/python/cp312-cp312/bin/pip install --no-cache-dir "cmake>=3.28,<4" ninja \
    && ln -s /opt/python/cp312-cp312/bin/cmake /usr/local/bin/cmake \
    && ln -s /opt/python/cp312-cp312/bin/ninja /usr/local/bin/ninja \
    && cmake --version && ninja --version && gcc --version | head -1

# Rust épinglé : la version du poste de développement au 10 octobre 2026.
ENV RUSTUP_HOME=/opt/rustup CARGO_HOME=/opt/cargo PATH=/opt/cargo/bin:$PATH
RUN curl -sSf https://sh.rustup.rs | sh -s -- -y --no-modify-path --profile minimal --default-toolchain ${RUST_VERSION} \
    && rustc --version && cargo --version

# Le cache de cargo (registre, dépôts git des dépendances, target) vit hors de
# l'image, dans /cache : un second bâti ne retélécharge ni ne recompile tout.
ENV CARGO_HOME=/cache/cargo
WORKDIR /src
ENTRYPOINT ["/bin/bash"]
