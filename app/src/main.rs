// Pas de console noire derrière la fenêtre en version finale.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::time::Instant;

fn main() -> iced::Result {
    let started = Instant::now();
    let args: Vec<String> = std::env::args().skip(1).collect();
    let bench = args.iter().any(|a| a == "--mesures");
    let demo = args
        .iter()
        .position(|a| a == "--demo")
        .map(|i| args.get(i + 1).and_then(|n| n.parse().ok()).unwrap_or(2000));
    launcher_ui::run(launcher_ui::Options {
        started,
        bench,
        demo,
    })
}
