fn main() -> anyhow::Result<()> {
    sensez::docs::write_site(std::env::current_dir()?)
}
