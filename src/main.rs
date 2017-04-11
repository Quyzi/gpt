#[macro_use]
extern crate clap;

use clap::{Arg, App};

mod gpt;

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

	let h = gpt::read_header2(&filename);
	println!("{:?}", h);
	println!("");
}