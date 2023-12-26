#!/bin/bash

export DEBIAN_FRONTEND=noninteractive

function install_qemu() {
    apt-get -y update
    apt-get -y install qemu-system
}

check=`which /usr/bin/qemu-system-x86_64`
if [ ''$check == '' ]
then
    install_qemu
else
    echo "qemu docker already install"
fi
