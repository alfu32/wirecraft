#[tokio::main]
async fn main() -> anyhow::Result<()> {
    wirecraft_runtime::cli::run(std::env::args().skip(1)).await
}
