use std::error::Error;

use windfinder::WindFinder;

pub mod windfinder;



fn main() -> Result<(), Box<dyn Error>> {

    let wf = WindFinder::new();

    let url = "https://www.windfinder.com/forecast/els_poblets_valencia_spain";

    let data = wf.scrape(url)?;

    println!("{}", data);

    Ok(())

}
