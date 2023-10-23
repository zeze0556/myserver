# myserver
myserver的后端，主要目的是想方便的使用bcachefs来作为nas系统的底层文件系统，但又不甘心
仅仅只做一个bcachefs的页面管理。功能配合myserver_web慢慢添加吧。

另一个目的是想用rust来弄个项目练练手，体验下rust生态的成熟度如何。

## 构建
``` bash
make
```

## 运行
```bash
make test
```
依赖下列软件:
1. sysstat(iostat)
查看IO设备读写速度等信息
2. bcachefs
3. samba
4. lsblk
获取硬盘信息
