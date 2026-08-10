#[path = "../machine_outcome.rs"]
mod machine_outcome;
#[path = "../production_outcome.rs"]
mod production_outcome;
#[path = "../px.rs"]
mod px;

fn main() {
    let path = std::env::args().nth(1).expect("case file path required");
    match production_outcome::report_json(&path) {
        Ok((report, true)) => println!("{}", report),
        Ok((report, false)) => {
            println!("{}", report);
            std::process::exit(1);
        }
        Err(error) => {
            eprintln!("pnix-production-outcome-check: {}", error);
            std::process::exit(1);
        }
    }
}
