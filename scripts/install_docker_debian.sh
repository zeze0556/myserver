#!/bin/bash

export DEBIAN_FRONTEND=noninteractive

function install_docker() {
    for pkg in docker.io docker-doc docker-compose podman-docker containerd runc; do sudo apt-get remove -y $pkg; done
    # Add Docker's official GPG key:
    sudo apt-get update -y
    sudo apt-get install -y ca-certificates curl gnupg
    sudo install -y -m 0755 -d /etc/apt/keyrings
    curl -fsSL https://download.docker.com/linux/debian/gpg | sudo gpg --yes --dearmor -o /etc/apt/keyrings/docker.gpg
    sudo chmod a+r /etc/apt/keyrings/docker.gpg

    # Add the repository to Apt sources:
    echo \
        "deb [arch=$(dpkg --print-architecture) signed-by=/etc/apt/keyrings/docker.gpg] https://download.docker.com/linux/debian \
  $(. /etc/os-release && echo "$VERSION_CODENAME") stable" | \
        sudo tee /etc/apt/sources.list.d/docker.list > /dev/null
    sudo apt-get update -y
    sudo apt-get install -y docker-ce docker-ce-cli containerd.io docker-buildx-plugin docker-compose-plugin
}

#function install_docker_compose() {
#    curl -L "https://github.com/docker/compose/releases/download/1.29.2/docker-compose-$(uname -s)-$(uname -m)" -o /usr/local/bin/docker-compose
#
#    chmod a+x /usr/local/bin/docker-compose
#}

check=`which docker`
if [ ''$check == '' ]
then
   install_docker
else
    echo "docker already install"
fi

#check=`which docker-compose`
#
#if [ ''$check == '' ]
#then
#    install_docker_compose
#else
#    echo "docker_compose already install"
#fi
usermod -aG docker $USER

service docker start
