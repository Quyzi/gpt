# GPT [![Build Status](https://travis-ci.org/Quyzi/gpt.svg?branch=master)](https://travis-ci.org/Quyzi/gpt)
[![Docs](https://docs.rs/gpt/badge.svg)](https://docs.rs/gpt)

Rust library for reading GPT headers and partition tables on disks and disk images. 

```rust
extern crate gpt;
use gpt::header::{Header, read_header};
use gpt::partition::{Partition, read_partitions};

let filename = "/dev/sda";
let mut h = read_header(&filename).unwrap();
let p = read_partitions(&filename, &mut h);
```
