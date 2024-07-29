use std::{collections::HashMap, str::FromStr};
use std::io;
use jup_ag::{Error, QuoteConfig};
use solana_client::rpc_client::RpcClient;
use solana_sdk::{
    commitment_config::CommitmentConfig, 
    pubkey::Pubkey
};
use spl_token::{
    solana_program::program_pack::Pack,
    state::Mint,
    amount_to_ui_amount,
    ui_amount_to_amount
};
use std::env;

mod utils;
use utils::price;


#[tokio::main]
async fn main() -> jup_ag::Result<()> {
    let route_map = jup_ag::route_map().await?;

    println!("Please input your start token:");
    let mut input_start: String = String::new();
    io::stdin().read_line(&mut input_start).expect("Failed to read input!");
    // input_start = String::from("DezXAZ8z7PnrnRJjz3wXBoRgixCa6xjnB7YaB1pPB263"); // For testing

    println!("Please input your end token:");
    let mut input_end: String = String::new();
    io::stdin().read_line(&mut input_end).expect("Failed to read input!");
    // input_end = String::from("So11111111111111111111111111111111111111112"); // For testing
    
    let mut input_target_hops: String = String::new();
    println!("Please input target hops on your route:");
    io::stdin().read_line(&mut input_target_hops).expect("Failed to read input!");
    let target_hops: usize = input_target_hops.trim().parse().expect("Please type a number!");


    let start_pubkey = Pubkey::from_str(input_start.trim()).unwrap_or_else(|err| {
        eprintln!("Failed to parse start token: {}", err);
        Pubkey::default()
    });

    let end_pubkey = Pubkey::from_str(input_end.trim()).unwrap_or_else(|err| {
        eprintln!("Failed to parse end token: {}", err);
        Pubkey::default()
    });

    let pubkeys: Vec<Pubkey> = vec![start_pubkey, end_pubkey];

    let (quote_info, total_result) = get_quote_info_for_route(&pubkeys).await.unwrap();
    println!("{}", quote_info);

    // Find all routes between start_pubkey and end_pubkey
    let mut all_routes = find_all_routes(&route_map, start_pubkey, end_pubkey, target_hops);

    // Find and output optimal routes based off quotes
    process_routes(&mut all_routes, total_result).await;

    Ok(())
}

async fn process_routes(all_routes: &mut Vec<Vec<Pubkey>>, biggest_result: f64) {
    let mut to_remove = Vec::new();
    
    for (index, route) in all_routes.iter().enumerate() {
        match get_quote_info_for_route(route).await {
            Ok((quote_info, total_result)) => {
                if total_result < biggest_result {
                    to_remove.push(index);
                } else {
                    println!("Yield more output token from the following route: ");
                    println!("{:?}", route);
                    println!("{}", quote_info);
                }
            },
            Err(_) => {
                to_remove.push(index);
            }
        }
    }

    // Remove items from the end to start to avoid index shifting issues
    for index in to_remove.into_iter().rev() {
        all_routes.remove(index);
    }
}


fn find_all_routes(
    route_map: &HashMap<Pubkey, Vec<Pubkey>>,
    start: Pubkey,
    end: Pubkey,
    max_hops: usize,
) -> Vec<Vec<Pubkey>> {
    let mut routes = Vec::new();
    let mut path = Vec::new();
    let mut visited = HashMap::new();

    find_routes_helper(route_map, start, end, max_hops, 0, &mut path, &mut routes, &mut visited);

    routes
}

// Recusive function that takes in a target depth and builds out paths given the route map
fn find_routes_helper(
    route_map: &HashMap<Pubkey, Vec<Pubkey>>,
    current: Pubkey,
    end: Pubkey,
    target_depth: usize,
    depth: usize,
    path: &mut Vec<Pubkey>,
    routes: &mut Vec<Vec<Pubkey>>,
    visited: &mut HashMap<Pubkey, bool>,
) {
    if depth > target_depth {
        return;
    }

    visited.insert(current, true);
    path.push(current);

    if current == end {
        routes.push(path.clone());
    } else if let Some(neighbors) = route_map.get(&current) {
        for &neighbor in neighbors {
            if !visited.get(&neighbor).unwrap_or(&false) {
                find_routes_helper(route_map, neighbor, end, target_depth, depth + 1, path, routes, visited);
            }
        }
    }

    path.pop();
    visited.insert(current, false);
}

async fn get_quote_info_for_route(
    path: &Vec<Pubkey>,
) -> jup_ag::Result<(String, f64)> {
    if path.len() < 2 {
        return Err(Error::JupiterApi("Path must contain at least two tokens".into()));
    }

    let ui_amount = 1.0;  // Example fixed amount for quotes
    let mut quote_details = String::new();
    let mut total_output = 1.;

    // Segment the path into pairs. For each pair
    // 1. Normalize the conversion rate
    // 2. Compare normalized conversion rate 
    // 3. Mulitply conversion rate to the final output
    for window in path.windows(2) {
        let start_pubkey = window[0];
        let end_pubkey = window[1];
        
        // Can use data for symbol info
        // let data = price(start_pubkey, end_pubkey, ui_amount).await?;

        // Fetch token details such as decimals and symbol, assuming a function `get_token_details`
        let (start_token, start_decimals) = (&start_pubkey.to_string() , get_token_details(&start_pubkey).await?);
        let (end_token, end_decimals) = (&end_pubkey.to_string() , get_token_details(&end_pubkey).await?);
 
        // Configure quote parameters
        let slippage_bps = 100;
        let only_direct_routes = true;
        let quotes = jup_ag::quote(
            start_pubkey,
            end_pubkey,
            ui_amount_to_amount(ui_amount, start_decimals),
            QuoteConfig {
                only_direct_routes,
                slippage_bps: Some(slippage_bps),
                ..QuoteConfig::default()
            },
        )
        .await?;

        let mut best_rate = 0.;
        let mut best_detail = String::new();

        quote_details += &format!("Received {} quotes between {} and {}:\n", quotes.route_plan.len(), start_token, end_token);
        for (i, quote) in quotes.route_plan.into_iter().enumerate() {
            let route = quote
                .swap_info
                .label
                .unwrap_or_else(|| "Unknown DEX".to_string());

            let out_amount = amount_to_ui_amount(quote.swap_info.out_amount, end_decimals);
            let in_amount = amount_to_ui_amount(quote.swap_info.in_amount, start_decimals);
            let normalized_rate =  out_amount / in_amount;
            
            let detail = format!(
                "{}. {} {} for {} {} via {} (worst case with slippage: {}). Impact: {:.2}% (NA: {})\n",
                i,
                in_amount,
                start_token,
                out_amount,
                end_token,
                route,
                amount_to_ui_amount(quotes.other_amount_threshold, end_decimals),
                quotes.price_impact_pct * 100.,
                normalized_rate
            );
            if normalized_rate > best_rate {
                best_rate = normalized_rate;
                best_detail = detail;
            }
            // quote_details += detail;
        }
        quote_details += &best_detail;
        quote_details += "\n";  // Separate each hop's quotes for clarity
        total_output *= best_rate;
    }

    Ok((quote_details, total_output))
}

async fn get_token_details(mint_pubkey: &Pubkey) -> jup_ag::Result<u8> {
    // Connect to Solana cluster
    let rpc_url = String::from(rpc_url());
    let client = RpcClient::new_with_commitment(rpc_url, CommitmentConfig::confirmed());

    // Fetch account data
    let account_data = match client.get_account_data(&mint_pubkey){
        Ok(account_data) => account_data,
        Err(e) => {
            return  Err(Error::JupiterApi(e.to_string()));
        }
    };

    // Deserialize the account data into a Mint object
    let mint = match Mint::unpack(&account_data){
        Ok(mint) => mint,
        Err(e) => {
            return  Err(Error::JupiterApi(e.to_string()));
        }
    };

    Ok(mint.decimals)
}

fn rpc_url() -> String {
    env::var("RPC_URL").unwrap_or_else(|_| "https://api.mainnet-beta.solana.com".to_string())
}
