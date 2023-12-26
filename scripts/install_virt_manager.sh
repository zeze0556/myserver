#!/bin/bash

export DEBIAN_FRONTEND=noninteractive
cmd=${1:-check}

function install_virt_manager() {
    #apt-get -y update
    #apt-get -y install virt-manager
    docker pull mber5/virt-manager
}

function run_virt_manager() {
    mkdir -p ../docker/virt-manager
    if [ ! -f "../docker/virt-manager/docker-compose.yml" ]; then
        cat <<EOL > "../docker/virt-manager/docker-compose.yml"
services:
  virt-manager:
    image: mber5/virt-manager:latest
    restart: always
    ports:
      - 8185:80
    environment:
      # Set DARK_MODE to true to enable dark mode
      DARK_MODE: false

      # Set HOSTS: "['qemu:///session']" to connect to a user session
      HOSTS: "['qemu:///system']"

      # If on an Ubuntu host (or any host with the libvirt AppArmor policy,
      # you will need to use an ssh connection to localhost
      # or use qemu:///system and uncomment the below line

      # privileged: true

    volumes:
      - "/var/run/libvirt/libvirt-sock:/var/run/libvirt/libvirt-sock"
      - "/var/lib/libvirt/images:/var/lib/libvirt/images"
EOL
    fi
    cd ../docker/virt-manager && docker compose up -d
}

function check() {
    #virt-manager
    local check_ret=`docker images | grep virt-manager`
    if [ "${check_ret}" == '' ]
    then
        echo "not installed"
    fi
    local check_run=`docker ps | grep virt-manager`
    if [ "${check_run}" == "" ]
    then
        echo "not run"
    fi

}

$cmd
