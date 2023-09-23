use pyrogen_parser;

pub fn print_message() {
    let num = 10;
    println!(
        "Hello, world! {num} plus one is {}!",
        pyrogen_parser::add_one(num)
    );
}
