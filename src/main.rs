#[macro_use]
extern crate clap;

extern crate gpt;

use clap::{Arg, App};
use gpt::header::{Header, read_header};
use gpt::partition::{Partition, read_partitions};

fn main()
{
	let input = App::new("sheep")
		.version(crate_version!())
		.author(crate_authors!())
		.about("Sheep")
		.arg(Arg::with_name("filename")
			.short("f")
			.help("Input filename")
			.required(true)
			.takes_value(true))
		.get_matches();

	let filename = input.value_of("filename").unwrap().to_string();

	let mut h = read_header(&filename).unwrap();
	let p = read_partitions(&filename, &mut h);

	println!("{:?}", h);
	println!("");
	println!("{:?}", p);
	println!("");
}