use colored::Colorize;
use tabled::{
    Table, Tabled,
    builder::Builder,
    settings::{Alignment, Modify, Style, object::Columns},
};

pub fn build_table<T: Tabled>(data: &[T]) -> String {
    if data.is_empty() {
        return "(no data)".dimmed().to_string();
    }
    let mut table = Table::new(data);
    table.with(Style::modern_rounded());
    table.with(Modify::new(Columns::new(1..)).with(Alignment::left()));
    table.to_string()
}

pub fn build_table_from_rows(headers: Vec<String>, rows: Vec<Vec<String>>) -> String {
    if rows.is_empty() {
        return "(no data)".dimmed().to_string();
    }
    let mut builder = Builder::default();
    builder.push_record(headers);
    for row in rows {
        builder.push_record(row);
    }
    let mut table = builder.build();
    table.with(Style::modern_rounded());
    table.with(Modify::new(Columns::new(1..)).with(Alignment::left()));
    // Optionally style first row as header (tabled handles first push_record as header in some styles)
    table.to_string()
}

pub fn print_success(msg: &str) {
    println!("{}", format!("✓ {}", msg).green().bold());
}

pub fn print_error(msg: &str) {
    eprintln!("{}", format!("✗ {}", msg).red().bold());
}

pub fn print_warning(msg: &str) {
    println!("{}", format!("⚠ {}", msg).yellow());
}

#[allow(dead_code)]
pub fn print_info(msg: &str) {
    println!("{}", format!("ℹ {}", msg).blue());
}

pub fn print_highlight(label: &str, value: &str) {
    println!("{}: {}", label.bold(), value.bright_yellow().bold());
}
