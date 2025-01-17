FROM golang:latest

MAINTAINER "--==RIX==--" <zeze0556@gmail.com>

env DEBIAN_FRONTEND=noninteractive
env TZ="Asia/Shanghai"

#RUN sed -i "s/archive.ubuntu.com/mirrors.ustc.edu.cn/g" /etc/apt/sources.list

RUN apt update -y && apt install -y \
    make \
    libbsd-dev \
    gcc \
    g++ \
    git \
    libc-dev \
    curl \
    && apt -y clean

