use rustpython_parser::{ast, Parse};

pub fn add_one(x: i32) -> i32 {
    x + 1
}

pub fn parser_test() {
    let python_source = r#"
def is_odd(i):
  return bool(i & 1)
"#;
    let ast = ast::Suite::parse(python_source, "<embedded>");
    println!("{:?}", ast)
}
