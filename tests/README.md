## How to create a gpt disk with `dd` and `fdisk`.
```dd if=/dev/zero of=tests/fixtures/gpt-disk.img bs=512 count=72```
```
sudo fdisk tests/fixtures/gpt-disk.img
g
n, 1, 34, 34
n, 2, 35, 38
w
```

```
sgdisk -i 1 tests/fixtures/gpt-disk.img
```

```
pip install gpt
```

Print mbr
```
sudo dd if=tests/fixtures/gpt-disk.img bs=512 count=1 skip=0 | print_mbr
```

Print primary gpt
```
sudo dd if=tests/fixtures/gpt-disk.img bs=512 count=1 skip=1 | print_gpt_header
```