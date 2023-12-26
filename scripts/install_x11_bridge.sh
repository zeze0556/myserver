#!/bin/bash

cmd=${1:-check}

function install_x11_bridge()
{
    docker pull jare/x11-bridge
}

function run_x11_bridge()
{
    local check=`docker ps | grep x11-bridge`
    if [ "${check}" == "" ]
    then
        docker run -d \
               --name x11-bridge \
               -e MODE="tcp" \
               -e XPRA_HTML="yes" \
               -e DISPLAY=:14 \
               -e XPRA_PASSWORD=111 \
               --net=host \
               jare/x11-bridge
    fi
}

function check() {
    local check_ret=`docker images | grep x11-bridge`
    if [ "${check_ret}" == '' ]
    then
        echo "not installed"
    fi
    local check_run=`docker ps | grep x11-bridge`
    if [ "${check_run}" == "" ]
    then
       echo "not run"
    fi
}

$cmd
