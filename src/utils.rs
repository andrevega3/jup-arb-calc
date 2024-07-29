use {
    serde::Deserialize,
    solana_sdk::pubkey::Pubkey,
    std::{collections::HashMap, env, str::FromStr},
    reqwest,
    jup_ag::{Error, Result},
};

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PriceData {
    #[serde(with = "pubkey_as_string", rename = "id")]
    pub input_mint: Pubkey,
    #[serde(rename = "mintSymbol")]
    pub input_symbol: String,
    #[serde(with = "pubkey_as_string", rename = "vsToken")]
    pub output_mint: Pubkey,
    #[serde(rename = "vsTokenSymbol")]
    pub output_symbol: String,
    pub price: f64,
}

#[derive(Clone, Debug, Deserialize)]
pub struct PriceResponse {
    pub data: HashMap<String, PriceData>,
    #[serde(rename = "timeTaken")]
    pub time_taken: f64,
}

mod pubkey_as_string {
    use std::str::FromStr;

    use serde::{self, Deserialize, Deserializer, Serializer};
    use solana_sdk::pubkey::Pubkey;

    pub fn serialize<S>(pubkey: &Pubkey, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&pubkey.to_string())
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Pubkey, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Pubkey::from_str(&s).map_err(serde::de::Error::custom)
    }
}

fn maybe_jupiter_api_error<T>(value: serde_json::Value) -> Result<T>
where
    T: serde::de::DeserializeOwned,
{
    #[derive(Deserialize)]
    struct ErrorResponse {
        error: String,
    }
    if let Ok(ErrorResponse { error }) = serde_json::from_value::<ErrorResponse>(value.clone()) {
        Err(Error::JupiterApi(error))
    } else {
        serde_json::from_value(value).map_err(|err| err.into())
    }
}

fn price_api_url() -> String {
    env::var("PRICE_API_URL").unwrap_or_else(|_| "https://price.jup.ag/v6".to_string())
}

/// https://price.jup.ag/v6/price?ids=DezXAZ8z7PnrnRJjz3wXBoRgixCa6xjnB7YaB1pPB263&vsToken=So11111111111111111111111111111111111111112&amount=1
/// Get simple price for a given input mint, output mint, and amount
pub async fn price(input_mint: Pubkey, output_mint: Pubkey, ui_amount: f64) -> Result<PriceData> {
    let url = format!(
        "{base_url}/price?ids={input_mint}&vsToken={output_mint}&amount={ui_amount}",
        base_url = price_api_url(),
    );
    // println!("{}", url);

    let response = reqwest::get(url).await?;
    let json: serde_json::Value = response.json().await?;
    // println!("Raw JSON response: {:?}", json);

    let price_response: PriceResponse = maybe_jupiter_api_error(json)?;
    let key = input_mint.to_string();
    let price_data = price_response.data.get(&key).ok_or_else(|| Error::JupiterApi(format!("Price data not found for key {}", key)))?;
    Ok(price_data.clone())
}
