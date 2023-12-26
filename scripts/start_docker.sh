#/bin/bash
set -ex
docker_config=$(cat /app/config/docker.config)
save_config=$(echo "$docker_config" | jq -r '.save_config')
save_type=$(echo "$save_config" | jq -r '.type')
# 构建镜像源配置
mirror_config=$(echo "$config" | jq -c '.mirror')
registry_mirrors="[]"
if [ "$mirror_config" != "[]" ]; then
    registry_mirrors=$(echo "$mirror_config" | jq -r 'map(.mirror) | join(", ")')
fi
docker_path="/var/lib/docker"

# 定义函数用于处理label数组
process_label() {
    local labels=$1
    local devices_str=""

    # 循环处理每个label
    devices_str=$(echo "$labels" | while read -r label; do
                      path=$(echo "$label" | jq -r '.path')
                      model=$(echo "$label" | jq -r '.model')

                      # 查找设备路径
                      device_path=$(lsblk -no NAME,MODEL | awk -v model="$model" '$2 == model {print "/dev/"$1}')

                      if [ -n "$device_path" ]; then
                          # 拼接以:为间隔的设备路径字符串
                          echo "${device_path}:"
                      else
                          echo "找不到硬盘 '$model' 的设备路径"
                          exit 1
                      fi
                  done)
    # 去除字符串末尾的冒号
    devices_str=$(echo "$devices_str" | sed 's/:$//')

    # 返回结果
    echo "$devices_str"
}

function check_mount_status()
{
    status=$(bcachefs fs usage $1 | grep "Filesystem:")
    echo $status
}

function mount_pool()
{
    pool_name=$1
    #echo "mount_pool $pool_name"
    config=$(cat /app/config/pools.config)
    auto_mount_true_items=$(echo "$config" | jq -c '.[] | select(.name== "'$pool_name'")')
    echo "$auto_mount_true_items" | while read -r item; do
        mount_path=$(echo "$item" | jq -r '.mount_path')
        labels=$(echo "$item" | jq -c '.label[]')

        # 调用函数处理label数组，并获取结果
        status=$(check_mount_status $mount_path)
        case "$status" in
            "")
                devices_str=$(process_label "$labels")
                # 挂载bcachefs文件系统
                mount -t bcachefs "$devices_str" "$mount_path"
                echo $mount_path
                ;;
            *)
                echo $mount_path
                ;;
        esac
    done

}

function set_up_docker_img()
{
    mount_path=$1
    img_path=$2
    cur=$(df | grep "/var/lib/docker" | awk '{print $1}')
    case $cur in
        /dev/loop*)
            echo "/var/lib/docker"
            ;;
        *)
            freeloop=$(losetup -f)
            losetup $freeloop "$mount_path/$img_path"
            mount $freeloop /var/lib/docker
            echo "/var/lib/docker"
            ;;
    esac
}
case "$save_type" in
    "pool")
        echo "process pool"
        pool_name=$(echo "$save_config" | jq -r '.pool_name')
        mount_path=$(mount_pool $pool_name)
        pool_config=$(echo "$save_config" | jq -r '.save')
        case $(echo "$pool_config" | jq -r ".type") in
            "pool_img")
                docker_path=$(set_up_docker_img $mount_path $(echo "$pool_config" | jq -r ".config.path"))
                ;;
            "pool_path")
                docker_path=$(echo "$save_config" | jq -r '.config.path')
                ;;
        esac
    ;;
    "disk_path")
        docker_path=$(echo "$save_config" | jq -r '.path')
        ;;
esac
# 构建最终输出
output=$(cat <<EOF
{
    "data-root": "$docker_path",
    "log-driver": "none",
    "bip": "172.10.0.1/24",
    "registry-mirrors": [$registry_mirrors]
}
EOF
)
echo $output > /etc/docker/daemon.json

systemctl restart docker.service
