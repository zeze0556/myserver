#!/bin/bash
# 解析JSON配置文件
config=$(cat /app/config/pools.config)
auto_mount_true_items=$(echo "$config" | jq -c '.[] | select(.auto_mount == true)')

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
# 循环处理每个auto_mount为true的配置项
echo "$auto_mount_true_items" | while read -r item; do
    mount_path=$(echo "$item" | jq -r '.mount_path')
    labels=$(echo "$item" | jq -c '.label[]')

    # 调用函数处理label数组，并获取结果
    devices_str=$(process_label "$labels")
    echo "device=$device_path"
    # 挂载bcachefs文件系统
    mount -t bcachefs "$devices_str" "$mount_path"
done
