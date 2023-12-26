#!/bin/bash

export DEBIAN_FRONTEND=noninteractive

function install_docker() {
    apt-get -y remove docker docker-engine docker.io containerd runc
    apt-get -y update
    apt-get -y install \
            ca-certificates \
            curl \
            gnupg \
            lsb-release \
            tgt

    curl -fsSL https://download.docker.com/linux/ubuntu/gpg | gpg --dearmor  -o /usr/share/keyrings/docker-archive-keyring.gpg

    echo \
        "deb [arch=$(dpkg --print-architecture) signed-by=/usr/share/keyrings/docker-archive-keyring.gpg] https://download.docker.com/linux/ubuntu \
  $(lsb_release -cs) stable" | tee /etc/apt/sources.list.d/docker.list > /dev/null

    apt-get update -y
    apt-get install docker-ce docker-ce-cli containerd.io docker-compose-plugin -y

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
