use clap::Parser;


#[derive(Parser)]
#[clap(version, author, about, long_about = None)]
struct CliArgs {
    
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = CliArgs::parse();

    Ok(())
}
