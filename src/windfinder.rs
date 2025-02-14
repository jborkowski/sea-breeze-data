use chrono::{DateTime, Duration, FixedOffset};
use chrono_tz::Tz;
use reqwest::blocking::Client;
use scraper::{Html, Selector};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{borrow::Cow, error::Error};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Data {
    datetime: DateTime<FixedOffset>,
    wind_direction: String,
    wind_status: String,
    wind_speed: f64,
    wave_direction: Option<String>,
    wave_period: Option<i32>,
    wave_height: Option<f64>,
    air_temperature: i32,
    spot_name: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct WindData {
    data: Vec<Data>,
}

impl WindData {
    pub fn for_date(&self, dt: DateTime<Tz>) -> Option<&Data> {
        self.data
            .iter()
            .find(|p| p.datetime > dt && p.datetime <= dt + Duration::hours(2))
            .take()
            .or_else(|| self.data.first())
    }
}

pub struct WindFinder {
    headers: reqwest::header::HeaderMap,
}

impl WindFinder {
    pub fn new() -> Self {
        let headers = reqwest::header::HeaderMap::new();
        // headers.insert("authority", "es.windfinder.com".parse().unwrap());
        WindFinder { headers }
    }

    fn beach_request(&self, url: &str) -> Result<String, Box<dyn Error>> {
        let client = Client::new();
        let response = client.get(url).headers(self.headers.clone()).send()?;
        Ok(response.text()?)
    }

    fn parse_wave_directions(&self, fetched_list: &[Value]) -> Vec<String> {
        fetched_list
            .iter()
            .map(|element| {
                let angle = element["wad"].as_f64().unwrap_or(0.0);
                self.angle_to_direction(angle)
            })
            .collect()
    }

    fn angle_to_direction(&self, angle: f64) -> String {
        let directions = [
            "N", "NNE", "NE", "ENE", "E", "ESE", "SE", "SSE", "S", "SSW", "SW", "WSW", "W", "WNW",
            "NW", "NNW",
        ];
        let idx = ((angle / 22.5).round() as usize) % 16;
        directions[idx].to_string()
    }

    fn parse_wind_directions(&self, fetched_list: &[Value]) -> Vec<String> {
        fetched_list
            .iter()
            .map(|element| {
                let angle = element["wd"].as_f64().unwrap_or(0.0);
                self.angle_to_direction(angle)
            })
            .collect()
    }

    fn parse_wind_speeds(&self, fetched_list: &[Value]) -> Vec<f64> {
        fetched_list
            .iter()
            .map(|element| element["ws"].as_f64().unwrap_or(0.0))
            .collect()
    }

    fn parse_wind_status(
        &self,
        wave_directions: &Vec<String>,
        wind_directions: &Vec<String>,
    ) -> Vec<String> {
        wave_directions
            .iter()
            .zip(wind_directions.iter())
            .map(|(wave_dir, wind_dir)| self.get_wind_status(wave_dir, wind_dir))
            .collect()
    }

    fn parse_air_temperature(&self, document: &Html) -> Vec<i32> {
        let selector = Selector::parse("div.data-temp.data--major.weathertable__cell span.units-at").unwrap();

        document
            .select(&selector)
            .map(|element| {
                let text = element.text().collect::<String>();
                let freq = text
                    .split_whitespace()
                    .next()
                    .unwrap_or("0")
                    .parse::<i32>()
                    .unwrap_or(0);

                freq
            })
            .collect()
    }

    fn parse_wave_periods(&self, document: &Html) -> Vec<i32> {
        let selector = Selector::parse("div.data-wavefreq.data--minor.weathertable__cell").unwrap();

        document
            .select(&selector)
            .map(|element| {
                let text = element.text().collect::<String>();
                let freq = text
                    .split_whitespace()
                    .next()
                    .unwrap_or("0")
                    .parse::<i32>()
                    .unwrap_or(0);

                freq
            })
            .collect()
    }

    fn get_wind_status(&self, _wind_dir: &str, _wave_dir: &str) -> String {
        // TODO
        "status".to_string()
    }

    fn parse_wave_heights(&self, fetched_list: &[Value]) -> Vec<f64> {
        fetched_list
            .iter()
            .map(|element| element["wh"].as_f64().unwrap_or(0.0))
            .collect()
    }

    fn date_datestr_to_datetime(&self, date_string: &str) -> DateTime<FixedOffset> {
        DateTime::parse_from_rfc3339(date_string).expect("Failed to parse RFC 3339 string")
    }

    fn parse_spot_name(&self, document: &Html) -> String {
        let selector = Selector::parse("span#spotheader-spotname").unwrap();
        document
            .select(&selector)
            .next()
            .unwrap()
            .text()
            .collect::<Vec<_>>()
            .join("")
            .trim()
            .to_string()
    }

    fn obtain_data(&self, document: &Html) -> WindData {
        let script_selector = Selector::parse("script").unwrap();
        let mut fetched_list: Vec<Value> = Vec::new();

        for script in document.select(&script_selector) {
            let script_text = script.text().collect::<Vec<_>>().join("");
            if script_text.contains("window.ctx.push")
                && script_text.to_lowercase().contains("fcdata")
            {
                let split_texts: Vec<&str> = script_text.split("window.ctx.push(").collect();
                for split_text in split_texts {
                    let replaced_push: Cow<str> = split_text
                        .replace("window.ctx.push(", "")
                        .replace(");", "")
                        .into();
                    let without_push: Vec<&str> = replaced_push.split("fcData:").collect();
                    if without_push.len() > 1 {
                        let without_push_text = without_push[1]
                            .split("]")
                            .next()
                            .unwrap()
                            .trim()
                            .to_string()
                            + "]";
                        fetched_list =
                            serde_json::from_str(&without_push_text.replace(": null", ": 0"))
                                .unwrap();
                    }
                }
            }
        }

        let datetimes: Vec<DateTime<FixedOffset>> = fetched_list
            .iter()
            .map(|element| {
                let date_str = element["dtl"].as_str().unwrap();
                self.date_datestr_to_datetime(date_str)
            })
            .collect();

        let wave_heights: Vec<f64> = self.parse_wave_heights(&fetched_list);
        let wave_directions: Vec<String> = self.parse_wave_directions(&fetched_list);
        let wind_directions: Vec<String> = self.parse_wind_directions(&fetched_list);
        let wind_speeds: Vec<f64> = self.parse_wind_speeds(&fetched_list);
        let wind_statuses: Vec<String> = self.parse_wind_status(&wave_directions, &wind_directions);
        let wave_periods: Vec<i32> = self.parse_wave_periods(document);
        let total_records = datetimes.len();
        let spot_name = self.parse_spot_name(document);
        let air_temperatures = self.parse_air_temperature(document);

        let data: Vec<Data> = (0..total_records)
            .map(|i| Data {
                datetime: datetimes[i],
                wind_direction: wind_directions[i].clone(),
                wind_status: wind_statuses[i].clone(),
                wave_direction: wave_directions.get(i).cloned().clone(),
                wave_period: wave_periods.get(i).copied(),
                wave_height: wave_heights.get(i).copied(),
                wind_speed: wind_speeds[i],
                spot_name: spot_name.clone(),
                air_temperature: air_temperatures[i],
            })
            .collect();

        WindData { data }
    }

    pub fn scrape(&self, url: &str) -> Result<WindData, Box<dyn Error>> {
        let response = self.beach_request(url)?;
        let document = Html::parse_document(&response);
        Ok(self.obtain_data(&document))
    }
}

impl Default for WindFinder {
    fn default() -> Self {
        Self::new()
    }
}
