use std::error::Error;
use chrono::{Duration, Local};
use chrono_tz::Europe::Madrid;
use clap::{Parser, Subcommand};
use windfinder::WindFinder;

pub mod windfinder;

#[derive(Debug, Parser)]
#[command(name = "sea")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    Now,
}

fn main() -> Result<(), Box<dyn Error>> {
    let args = Cli::parse();

    let wf = WindFinder::new();

    let url = "https://www.windfinder.com/forecast/els_poblets_valencia_spain";

    let data = wf.scrape(url)?;

    match args.command {
        Commands::Now => {
            let now = Local::now().with_timezone(&Madrid);
            let result = data
                .data
                .iter()
                .find(|p| p.datetime > now && p.datetime <= now + Duration::hours(2))
                .take()
                // .or_else(|| data.data.first())
                .expect("There is no data");
            println!("{:?}", result);
        }
    }

    Ok(())
}
