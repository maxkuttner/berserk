mod berserk;
mod reader;

use tokio;
use reqwest;
use log::debug;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let chapters = berserk::get_all_chapters()
        .await?
        .iter()
        .map(|c| url::Url::parse(c))
        .collect::<Result<Vec<_>, _>>()?;

    reader::run(chapters).await?;

    Ok(())
}