extern crate ansi_term;
extern crate mini_v8;
extern crate rustyline;

use ansi_term::Colour::{Green, Red, Fixed};
use mini_v8::{MiniV8, Value, Error as MV8Error, Script, ScriptOrigin};
use rustyline::{Editor, error::ReadlineError};
use std::time::SystemTime;

fn main() {
    println!("Type \\h for help.\n");

    let mv8 = MiniV8::new();
    let mut rl = Editor::<()>::new();

    loop {
        match rl.readline(&"# ") {
            Ok(ref line) if line.starts_with("\\") => {
                let code = &line[1..line.len()];
                match code {
                    "h" => print_help(),
                    "q" => break,
                    _ => println!("Unknown command. Type \\h for help."),
                }

                rl.add_history_entry(line);
            },
            Ok(line) => {
                let before = SystemTime::now();
                let result: Result<Value, MV8Error> = mv8.eval(Script {
                    source: line.clone(),
                    origin: Some(ScriptOrigin { name: "repl".to_owned(), ..Default::default() }),
                });
                let elapsed = SystemTime::now().duration_since(before).unwrap();
                match result {
                    Ok(value) => print_value(value),
                    Err(error) => print_error(error.to_value(&mv8)),
                }
                println!("{}", Fixed(245).paint(&format!("Evaluated in {:?}", elapsed)));

                rl.add_history_entry(line);
            },
            Err(ReadlineError::Interrupted) => continue,
            Err(ReadlineError::Eof) => break,
            Err(err) => {
                println!("REPL error: {:?}", err);
                break;
            },
        }
    }
}

fn print_help() {
    println!("You are using a JavaScript REPL backed by the V8 engine.");
    println!("Type: \\q to quit");
    println!("      \\h for this dialog");
}

fn print_value(value: Value) {
    println!("{} {:?}", Green.paint("=>"), value);
}

fn print_error(error: Value) {
    println!("{} {:?}", Red.paint("!>"), error);
}
