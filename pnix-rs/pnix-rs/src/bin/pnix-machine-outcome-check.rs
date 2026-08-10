#[path = "../machine_outcome.rs"]
mod machine_outcome;

fn main() {
    println!("{}", machine_outcome::self_check_json());
}
