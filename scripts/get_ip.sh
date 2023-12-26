#!/bin/bash
#set -ex

# 获取所有网卡名称
all_interfaces=$(ls /sys/class/net/)

# 初始化空数组
output_array=()

# 循环处理每个网卡
for interface in $all_interfaces; do
    # 获取网卡设备目录
    device_path="/sys/class/net/$interface/device"

    # 检查是否存在PCI信息
    if [ -d "$device_path" ]; then
        pci_address=$(readlink "$device_path" | awk -F "/" '{print $4}')
        mac_address=$(cat "/sys/class/net/$interface/address")

        # 获取 IPv4 地址数组
        ipv4_addresses=($(ip -o -4 addr show dev "$interface" | awk '{print $4}' | cut -d'/' -f1))
        
        # 获取 IPv6 地址数组
        ipv6_addresses=($(ip -o -6 addr show dev "$interface" | awk '{print $4}' | cut -d'/' -f1))

        # 将信息添加到数组
        output_array+=("{\"interface\":\"$interface\",\"mac\":\"$mac_address\",\"pci_address\":\"$pci_address\",\"ipv4\":$(printf '%s\n' "${ipv4_addresses[@]}" | jq -R . -c -s),\"ipv6\":$(printf '%s\n' "${ipv6_addresses[@]}" | jq -R . -c -s)}")
    fi
done

# 将数组转换为JSON格式输出
json_output="[${output_array[*]}]"
echo "$json_output"
