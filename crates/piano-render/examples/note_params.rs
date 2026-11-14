//! A key's string and hammer as the engine takes them from a preset (analysis).
//!   cargo run --release -p piano-render --example note_params -- <preset.toml> note [note …]
fn main() {
    let a: Vec<String> = std::env::args().skip(1).collect();
    let model = piano_host::load_preset(std::path::Path::new(&a[0])).unwrap();
    for n in &a[1..] {
        let note: u8 = n.parse().unwrap();
        println!("{note}: {:?}", model.note(note));
    }
}
