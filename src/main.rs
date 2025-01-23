use serde::{Deserialize, Serialize};
use solana_account_decoder_client_types::UiAccountData;
use solana_client::rpc_client::RpcClient;
use solana_client::rpc_request::TokenAccountsFilter;
use solana_sdk::pubkey::Pubkey;
use std::collections::HashMap;
use std::fs;
use std::str::FromStr;

#[derive(Debug, Deserialize)]
struct TokenConfig {
    #[serde(default = "default_rpc_url")]
    solana_rpc_url: String,
    wallets: Vec<String>,
    tokens: Vec<TokenInfo>,
}

#[derive(Debug, Deserialize, Clone)]
struct TokenInfo {
    address: String,
    ticker: String,
}

#[derive(Debug, Serialize)]
struct BalanceResult {
    sol_balance: f64,
    token_balances: HashMap<String, f64>,
}

#[derive(Deserialize, Debug)]
struct ParsedInfo {
    info: AccountInfo,
}

#[derive(Deserialize, Debug)]
struct AccountInfo {
    #[serde(rename = "tokenAmount")]
    token_amount: TokenAmount,
}

#[derive(Deserialize, Debug)]
struct TokenAmount {
    #[serde(rename = "uiAmount")]
    ui_amount: Option<f64>,
}

fn default_rpc_url() -> String {
    "https://api.mainnet-beta.solana.com".to_string()
}

async fn get_wallet_balances(
    config: &TokenConfig,
) -> Result<HashMap<String, BalanceResult>, anyhow::Error> {
    let client = RpcClient::new(&config.solana_rpc_url);
    let mut results = HashMap::new();

    for wallet_str in &config.wallets {
        let wallet_pubkey = Pubkey::from_str(wallet_str)?;

        let sol_balance = client.get_balance(&wallet_pubkey)?;

        let token_balances = get_token_balances(&client, &wallet_pubkey, &config.tokens)?;

        results.insert(
            wallet_str.clone(),
            BalanceResult {
                sol_balance: sol_balance as f64 / 1_000_000_000.0,
                token_balances,
            },
        );
    }

    Ok(results)
}

fn get_token_balances(
    client: &RpcClient,
    wallet_pubkey: &Pubkey,
    tokens: &[TokenInfo],
) -> Result<HashMap<String, f64>, anyhow::Error> {
    let mut token_balances = HashMap::new();

    for token in tokens {
        let mint_pubkey = Pubkey::from_str(&token.address)?;

        let token_accounts = client
            .get_token_accounts_by_owner(wallet_pubkey, TokenAccountsFilter::Mint(mint_pubkey))?;

        dbg!(&token_accounts);
        let total_balance: f64 = token_accounts
            .iter()
            .filter_map(|account| match &account.account.data {
                UiAccountData::Json(parsed_account) => {
                    serde_json::from_value::<ParsedInfo>(parsed_account.parsed.clone())
                        .ok()?
                        .info
                        .token_amount
                        .ui_amount
                }
                _ => None,
            })
            .sum();
        // dbg!(&total_balance);

        token_balances.insert(token.ticker.clone(), total_balance);
    }

    Ok(token_balances)
}

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    let config_content = fs::read_to_string("config.yaml")?;
    let config: TokenConfig = serde_yaml::from_str(&config_content)?;

    let balances = get_wallet_balances(&config).await?;

    println!("Detailed Wallet Balances:");
    for (wallet, balance_info) in &balances {
        println!("Wallet: {}", wallet);
        println!("SOL Balance: {:.4} SOL", balance_info.sol_balance);

        println!("Token Balances:");
        for (token, amount) in &balance_info.token_balances {
            println!("  {}: {:.4}", token, amount);
        }
        println!();
    }

    Ok(())
}
