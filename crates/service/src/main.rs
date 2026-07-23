use std::io::{self, Read};

use skills_copilot_service::handle_request_json;

fn main() {
    skills_copilot_commands::initialize_action_preview_secret_from_environment();
    let mut input = String::new();
    if let Err(error) = io::stdin().read_to_string(&mut input) {
        eprintln!("failed to read request: {error}");
        std::process::exit(1);
    }
    if input.trim().is_empty() {
        input = r#"{"id":"status","method":"service.status","params":{}}"#.to_string();
    }
    println!("{}", handle_request_json(&input));
}
