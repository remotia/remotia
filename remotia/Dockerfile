# This file is a template, and might need editing before it works on your project.
FROM rust:1.56.1-buster

WORKDIR /usr/src/app

COPY . .

RUN apt-get update -qq
RUN apt-get -y install autoconf automake build-essential cmake \
            git-core libass-dev libfreetype6-dev libgnutls28-dev libsdl2-dev \
            libtool libva-dev libvdpau-dev libvorbis-dev libxcb1-dev \
            libxcb-shm0-dev libxcb-xfixes0-dev pkg-config texinfo wget yasm \
            zlib1g-dev
RUN apt-get -y install nasm libx264-dev libx265-dev libnuma-dev \
            libvpx-dev libmp3lame-dev libopus-dev

RUN bash utils/linux_ffmpeg.rs

ENTRYPOINT ["bash"]
