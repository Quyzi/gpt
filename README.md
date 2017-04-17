# GPT
[![Build Status](https://travis-ci.org/Quyzi/gpt.svg?branch=master)](https://travis-ci.org/Quyzi/gpt)

Rust library for reading GPT headers and partition tables on disks and disk images. 

```rust
extern crate gpt;

let filename = "/dev/sda";
let mut h = read_header(&filename).unwrap();
let p = read_partitions(&filename, &mut h);
```
