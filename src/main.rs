mod api;
mod app;
mod config;
mod runtime;
mod ui;

#[cfg(test)]
mod tests;

fn main() -> std::io::Result<()> {
    runtime::run()
}
