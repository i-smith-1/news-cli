use anyhow::Result;
use console::{style, Term};
use futures_util::future::join_all;
use reqwest::Client;
use serde_json::Value;

use crate::config::{RuntimeConfig, StatsConfig};

pub async fn run(cfg: &RuntimeConfig) -> Result<()> {
    let term = Term::stdout();
    let _ = term.clear_screen();

    let client = Client::builder()
        .user_agent("news-cli/0.1 stats")
        .gzip(true)
        .build()?;

    // Fetch in parallel
    let pol = fetch_boc_latest_number(&client, "V39079"); // Target for the overnight rate
    let cpi = fetch_boc_latest_number(&client, "STATIC_TOTALCPICHANGE"); // Total CPI, % change over 1 year ago

    let yields = fetch_yield_curve(&client, &cfg.stats).await;

    let (policy_rate, inflation) = futures_util::join!(pol, cpi);

    // Display
    println!("Key Stats (q = quit, b = back)");
    println!("");
    match policy_rate {
        Ok(Some(v)) => println!("- Policy rate (BoC): {:.2}%", v),
        Ok(None) => println!("- Policy rate (BoC): N/A"),
        Err(e) => println!("- Policy rate (BoC): error: {}", e),
    }
    match inflation {
        Ok(Some(v)) => println!("- Inflation YoY (CPI, BoC): {:.2}%", v),
        Ok(None) => println!("- Inflation YoY (BoC): N/A"),
        Err(e) => println!("- Inflation YoY (BoC): error: {}", e),
    }

    // Population (StatsCan) last 4 quarters, if configured
    if let Some(vec_id) = cfg.stats.statscan_population_vector.as_ref() {
        match fetch_statcan_last_n(&client, vec_id, 4).await {
            Ok(Some(points)) => {
                println!("- Population (StatsCan, last 4q):");
                for (period, val) in points {
                    println!("  {}: {}", period, val);
                }
            }
            Ok(None) => println!("- Population (StatsCan): N/A"),
            Err(e) => println!("- Population (StatsCan): error: {}", e),
        }
    } else {
        println!("- Population (StatsCan): not configured (add stats.statscan_population_vector)");
    }

    // Housing starts (StatsCan/CMHC) last 4 periods, if configured
    if let Some(vec_id) = cfg.stats.housing_starts_vector.as_ref() {
        match fetch_statcan_last_n(&client, vec_id, 4).await {
            Ok(Some(points)) => {
                println!("- Housing starts (StatsCan/CMHC, last 4):");
                for (period, val) in points {
                    println!("  {}: {}", period, val);
                }
            }
            Ok(None) => println!("- Housing starts: N/A"),
            Err(e) => println!("- Housing starts: error: {}", e),
        }
    } else {
        println!("- Housing starts: not configured (add stats.housing_starts_vector)");
    }

    // Yield curve
    println!("");
    println!("Yield Curve (BoC):");
    render_yield_curve_line(&yields);

    // Wait for user to go back or quit
    println!("");
    println!("Press Enter to return, 'q' to quit.");
    match term.read_key()? {
        console::Key::Char('q') | console::Key::Char('Q') => std::process::exit(0),
        _ => {}
    }

    Ok(())
}

async fn fetch_boc_latest_number(client: &Client, series: &str) -> Result<Option<f64>> {
    let url = format!(
        "https://www.bankofcanada.ca/valet/observations/{}?recent=1",
        series
    );
    let text = client.get(url).send().await?.text().await?;
    let v: Value = serde_json::from_str(&text)?;
    let obs = v.get("observations").and_then(|x| x.as_array());
    let Some(arr) = obs else { return Ok(None) };
    let Some(obj) = arr.last().and_then(|x| x.as_object()) else { return Ok(None) };
    // Prefer direct field by series id
    if let Some(val) = obj.get(series) {
        if let Some(s) = val.get("v").and_then(|x| x.as_str()) {
            if let Ok(n) = s.parse::<f64>() { return Ok(Some(n)); }
        }
        if let Some(s) = val.as_str() {
            if let Ok(n) = s.parse::<f64>() { return Ok(Some(n)); }
        }
    }
    // Fallback: scan values except the date field 'd'
    for (k, val) in obj.iter() {
        if k == "d" { continue; }
        if let Some(s) = val.get("v").and_then(|x| x.as_str()) {
            if let Ok(n) = s.parse::<f64>() { return Ok(Some(n)); }
        }
        if let Some(s) = val.as_str() {
            if let Ok(n) = s.parse::<f64>() { return Ok(Some(n)); }
        }
    }
    Ok(None)
}

async fn fetch_statcan_last_n(client: &Client, vector: &str, n: usize) -> Result<Option<Vec<(String, String)>>> {
    // StatsCan WDS REST API: POST getDataFromVectorsAndLatestNPeriods
    // Vector IDs are numeric; strip any leading 'v'/'V' prefix from config values
    let vec_id_str = vector.trim_start_matches(|c: char| c == 'v' || c == 'V');
    let vec_id: u64 = vec_id_str.parse()
        .map_err(|_| anyhow::anyhow!("invalid StatsCan vector id: {}", vector))?;

    let url = "https://www150.statcan.gc.ca/t1/wds/rest/getDataFromVectorsAndLatestNPeriods";
    let body = serde_json::json!([{"vectorId": vec_id, "latestN": n}]);
    let text = client.post(url).json(&body).send().await?.text().await?;
    let v: Value = serde_json::from_str(&text)?;

    // Response is an array: [{status, object: {vectorDataPoint: [...]}}]
    if let Some(first) = v.as_array().and_then(|a| a.first()) {
        if let Some(points) = first.get("object").and_then(|o| o.get("vectorDataPoint")).and_then(|x| x.as_array()) {
            let mut out: Vec<(String, String)> = Vec::new();
            for p in points {
                let period = p.get("refPer").and_then(|x| x.as_str()).unwrap_or("").to_string();
                // value is a JSON number
                let val = match p.get("value") {
                    Some(Value::Number(n)) => n.to_string(),
                    Some(Value::String(s)) => s.clone(),
                    _ => "".to_string(),
                };
                out.push((period, val));
            }
            return Ok(Some(out));
        }
    }
    Ok(None)
}

async fn fetch_yield_curve(client: &Client, stats: &StatsConfig) -> Vec<(String, Option<f64>)> {
    let default_series: Vec<(String, String)> = vec![
        ("3M".to_string(), "TB.CDN.90D.MID".to_string()),  // 3-month T-bill mid-rate
        ("2Y".to_string(), "BD.CDN.2YR.DQ.YLD".to_string()),  // GoC 2-year benchmark bond yield
        ("5Y".to_string(), "BD.CDN.5YR.DQ.YLD".to_string()),  // GoC 5-year benchmark bond yield
        ("10Y".to_string(), "BD.CDN.10YR.DQ.YLD".to_string()), // GoC 10-year benchmark bond yield
        ("Long".to_string(), "BD.CDN.LONG.DQ.YLD".to_string()), // GoC long-term benchmark bond yield
    ];
    let pairs: Vec<(String, String)> = match stats.boc_yield_series.as_ref() {
        Some(map) => {
            let mut v: Vec<(String, String)> = map.iter().map(|(k, s)| (k.clone(), s.clone())).collect();
            v.sort_by(|a, b| a.0.cmp(&b.0));
            v
        }
        None => default_series,
    };

    let futs = pairs.iter().map(|(_label, id)| fetch_boc_latest_number(client, id));
    let vals = join_all(futs).await;
    let mut out: Vec<(String, Option<f64>)> = Vec::new();
    for ((label, _), v) in pairs.into_iter().zip(vals.into_iter()) {
        out.push((label, v.ok().flatten()));
    }
    out
}

fn render_yield_curve_line(data: &[(String, Option<f64>)]) {
    if data.is_empty() {
        println!("(no yield data)");
        return;
    }
    // Build a single line with inversion coloring against previous point
    let mut prev: Option<f64> = None;
    let mut parts: Vec<String> = Vec::new();
    for (label, val) in data.iter() {
        match (val, prev) {
            (Some(v), Some(p)) => {
                let s = if *v < p { // inverted relative to previous maturity
                    format!("{}: {}%", label, style(format!("{:.2}", v)).red())
                } else {
                    format!("{}: {}%", label, style(format!("{:.2}", v)).green())
                };
                parts.push(s);
                prev = Some(*v);
            }
            (Some(v), None) => {
                parts.push(format!("{}: {}%", label, style(format!("{:.2}", v)).green()));
                prev = Some(*v);
            }
            (None, _) => {
                parts.push(format!("{}: N/A", label));
            }
        }
    }
    println!("- {}", parts.join(" | "));
}
