extern crate winres;

use std::fs;

fn main() {
    if cfg!(target_os = "windows") {
        let mut res = winres::WindowsResource::new();
        res.set_icon("assets/favicon.ico");
        res.compile().unwrap();
    }
    
    generate_facts("assets/eelfacts/normal.txt", "NORMAL_FACTS", "normal_facts");
    generate_facts("assets/eelfacts/sus.txt", "SUS_FACTS", "sus_facts");
    generate_facts("assets/eelfacts/amogus.txt", "AMOGUS_FACTS", "amogus_facts");
    generate_facts("assets/eelfacts/insanity.txt", "INSANITY_FACTS", "insanity_facts");

}

fn generate_facts(path: &str, name: &str, classname: &str) {
    let contents = fs::read_to_string(path).unwrap();

    let lines: Vec<String> = contents
        .lines()
        .map(|line| format!("    {:?},", line))
        .collect();


    let output = format!(
        "pub static {}: &[&str] = &[\n{}\n];", name,
        lines.join("\n")
    );

    fs::write(format!("src/{}.rs", classname), output).unwrap();
}