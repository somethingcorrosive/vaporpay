use clap::{Parser, Subcommand};
use rand::RngCore;
use sha3::{Digest, Keccak256};
use std::time::{SystemTime, UNIX_EPOCH};
use chrono::{Utc, TimeZone};
use std::str::FromStr;
use std::env;
use dotenv::dotenv;
use ethers::{
    prelude::*,
    abi::{Function, Param, ParamType, StateMutability, Token},
};
use std::sync::Arc;
use qrcode::QrCode;
use qrcode::types::Color;
use image::{ImageBuffer, Luma};

#[derive(Parser)]
#[command(name = "vaporpay")]
#[command(about = "Off-chain tools for the VaporPay smart contract", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Deposit ETH into a commitment
    Create {
        #[arg(long)]
        amount: f64,

        #[arg(long)]
        expiry: String,

        #[arg(long, action = clap::ArgAction::SetTrue)]
        refundable: bool,

        #[arg(long, action = clap::ArgAction::SetTrue)]
        send: bool,

        #[arg(long)]
        contract: Option<String>,

        #[arg(long, action = clap::ArgAction::SetTrue)]
        qr: bool,
    },

    /// Deposit ERC20 tokens
    DepositErc20 {
        #[arg(long)]
        token: String,

        #[arg(long)]
        amount: f64,

        #[arg(long, default_value_t = 18)]
        decimals: u8,

        #[arg(long)]
        expiry: String,

        #[arg(long, action = clap::ArgAction::SetTrue)]
        refundable: bool,

        #[arg(long, action = clap::ArgAction::SetTrue)]
        send: bool,

        #[arg(long)]
        contract: String,

        #[arg(long, action = clap::ArgAction::SetTrue)]
        qr: bool,

    },

    /// Redeem a secret + salt
    Redeem {
        #[arg(long)]
        secret: String,

        #[arg(long)]
        salt: String,

        #[arg(long)]
        to: String,

        #[arg(long)]
        contract: String,
    },

    /// Refund expired deposit
    Refund {
        #[arg(long)]
        secret: String,

        #[arg(long)]
        salt: String,

        #[arg(long)]
        contract: String,
    }
}

//#[derive(Serialize)]
//struct CommitmentData {
//    secret: String,
//    salt: String,
//    commitment: String,
//}

fn keccak256(hex_input: &[u8]) -> String {
    let hash = Keccak256::digest(hex_input);
    format!("0x{}", hex::encode(hash))
}

fn parse_duration(s: &str) -> anyhow::Result<u64> {
    let mut total = 0;
    let mut buf = String::new();
    let mut unit = String::new();

    for c in s.chars() {
        if c.is_digit(10) {
            if !unit.is_empty() {
                total += parse_single(&buf, &unit)?;
                buf.clear();
                unit.clear();
            }
            buf.push(c);
        } else {
            unit.push(c);
        }
    }

    if !buf.is_empty() && !unit.is_empty() {
        total += parse_single(&buf, &unit)?;
    }

    Ok(total)
}

fn parse_single(num: &str, unit: &str) -> anyhow::Result<u64> {
    let n = num.parse::<u64>()?;
    match unit {
        "s" => Ok(n),
        "m" => Ok(n * 60),
        "h" => Ok(n * 3600),
        "d" => Ok(n * 86400),
        _ => Err(anyhow::anyhow!("Unknown unit: {unit}")),
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv().ok();
    let cli = Cli::parse();

    match &cli.command {
        Commands::Create { amount, expiry, refundable, send, contract, qr } => {
            handle_create_eth(*amount, expiry, *refundable, *send, contract.clone(), *qr).await?;
        }
        Commands::DepositErc20 { token, amount, decimals, expiry, refundable, send, contract, qr } => {
            handle_deposit_erc20(token, *amount, *decimals, expiry, *refundable, *send, contract, *qr).await?;
        }
        Commands::Redeem { secret, salt, to, contract } => {
            handle_redeem(secret, salt, to, contract).await?;
        }
        Commands::Refund { secret, salt, contract } => {
            handle_refund(secret, salt, contract).await?;
        }
    }

    Ok(())
}

// ---------------- HANDLERS ----------------

async fn handle_create_eth(amount: f64, expiry: &str, refundable: bool, send: bool, contract: Option<String>, qr: bool) -> anyhow::Result<()> {
    let (secret_hex, salt_hex, commitment, expiry_unix, expiry_date) = make_commitment(expiry)?;
    println!("\nVaporPay ETH Commitment Generated!\n");
    println!("Secret:     {}", secret_hex);
    println!("Salt:       {}", salt_hex);
    println!("Commitment: {}", commitment);
    println!("Amount:     {} ETH", amount);
    println!("Expiry:     {} ({} from now)", expiry_date, expiry);
    println!("Refundable: {}", if refundable { "Yes" } else { "No" });

    if let Some(contract_addr) = &contract {
        if qr {
            generate_qr(&secret_hex, &salt_hex, contract_addr)?;
        }
    }

    if send {
        let contract_address = contract.expect("Missing --contract");
        let private_key = env::var("PRIVATE_KEY").expect("Missing PRIVATE_KEY");
        let rpc_url = env::var("RPC_URL").expect("Missing RPC_URL");

        let provider = Provider::<Http>::try_from(rpc_url)?;
        let chain_id = provider.get_chainid().await?.as_u64();
        let wallet: LocalWallet = private_key.parse::<LocalWallet>()?.with_chain_id(chain_id);
        let client = SignerMiddleware::new(provider, wallet);
        let client = Arc::new(client);

        let to_commitment = H256::from_str(&commitment)?;
        let value = ethers::utils::parse_ether(amount)?;

        println!("\n Sending depositETH transaction...");

        let tx = TransactionRequest::new()
            .to(Address::from_str(&contract_address).expect("Invalid contract address"))
            .value(value)
            .data(encode_deposit_eth(to_commitment, expiry_unix, refundable));

        let pending_tx = client.send_transaction(tx, None).await?;
        let receipt = pending_tx.await?;

        match receipt {
            Some(r) => println!("TX confirmed: https://sepolia.etherscan.io/tx/{:?}", r.transaction_hash),
            None => println!("TX sent but not confirmed."),
        }
    }

    Ok(())
}

async fn handle_deposit_erc20(token: &str, amount: f64, decimals: u8, expiry: &str, refundable: bool, send: bool, contract: &str, qr: bool) -> anyhow::Result<()> {
    let (secret_hex, salt_hex, commitment, expiry_unix, expiry_date) = make_commitment(expiry)?;
    println!("\nVaporPay ERC20 Commitment Generated!\n");
    println!("Secret:     {}", secret_hex);
    println!("Salt:       {}", salt_hex);
    println!("Commitment: {}", commitment);
    println!("Amount:     {} tokens ({} decimals)", amount, decimals);
    println!("Expiry:     {} ({} from now)", expiry_date, expiry);
    println!("Refundable: {}", if refundable { "Yes" } else { "No" });

    if qr {
        generate_qr(&secret_hex, &salt_hex, contract)?;
    }

    if send {
        let private_key = env::var("PRIVATE_KEY").expect("Missing PRIVATE_KEY");
        let rpc_url = env::var("RPC_URL").expect("Missing RPC_URL");

        let provider = Provider::<Http>::try_from(rpc_url)?;
        let chain_id = provider.get_chainid().await?.as_u64();
        let wallet: LocalWallet = private_key.parse::<LocalWallet>()?.with_chain_id(chain_id);
        let client = SignerMiddleware::new(provider, wallet);
        let client = Arc::new(client);

        let token_address = Address::from_str(token)?;
        let contract_address = Address::from_str(contract)?;
        let to_commitment = H256::from_str(&commitment)?;

        // Convert human-readable to base units
        let multiplier = 10u128.pow(decimals as u32);
        let value = U256::from((amount * multiplier as f64) as u128);

        println!("\nApproving token spend...");
        #[allow(deprecated)]
        let approve_fn = Function {
            name: "approve".to_string(),
            inputs: vec![
                Param { name: "spender".into(), kind: ParamType::Address, internal_type: None },
                Param { name: "amount".into(), kind: ParamType::Uint(256), internal_type: None },
            ],
            outputs: vec![Param { name: "".into(), kind: ParamType::Bool, internal_type: None }],
            state_mutability: StateMutability::NonPayable,
            constant: None,
        };
        let approve_data = approve_fn.encode_input(&[
            Token::Address(contract_address),
            Token::Uint(value),
        ])?;
        let approve_tx = TransactionRequest::new().to(token_address).data(approve_data);
        client.send_transaction(approve_tx, None).await?.await?;

        println!("\nSending depositERC20 transaction...");
        #[allow(deprecated)]
        let deposit_fn = Function {
            name: "depositERC20".to_string(),
            inputs: vec![
                Param { name: "commitment".into(), kind: ParamType::FixedBytes(32), internal_type: None },
                Param { name: "token".into(), kind: ParamType::Address, internal_type: None },
                Param { name: "amount".into(), kind: ParamType::Uint(256), internal_type: None },
                Param { name: "expiry".into(), kind: ParamType::Uint(256), internal_type: None },
                Param { name: "refundable".into(), kind: ParamType::Bool, internal_type: None },
            ],
            outputs: vec![],
            state_mutability: StateMutability::NonPayable,
            constant: None,
        };
        let deposit_data = deposit_fn.encode_input(&[
            Token::FixedBytes(to_commitment.as_bytes().to_vec()),
            Token::Address(token_address),
            Token::Uint(value),
            Token::Uint(expiry_unix.into()),
            Token::Bool(refundable),
        ])?;
        let deposit_tx = TransactionRequest::new().to(contract_address).data(deposit_data);
        let receipt = client.send_transaction(deposit_tx, None).await?.await?;

        match receipt {
            Some(r) => println!("ERC20 deposit TX confirmed: https://sepolia.etherscan.io/tx/{:?}", r.transaction_hash),
            None => println!("ERC20 deposit TX sent but not confirmed."),
        }
    }

    Ok(())
}

async fn handle_redeem(secret: &str, salt: &str, to: &str, contract: &str) -> anyhow::Result<()> {
    let private_key = env::var("PRIVATE_KEY").expect("Missing PRIVATE_KEY");
    let rpc_url = env::var("RPC_URL").expect("Missing RPC_URL");

    let provider = Provider::<Http>::try_from(rpc_url)?;
    let chain_id = provider.get_chainid().await?.as_u64();
    let wallet: LocalWallet = private_key.parse::<LocalWallet>()?.with_chain_id(chain_id);
    let client = SignerMiddleware::new(provider, wallet);
    let client = Arc::new(client);

    println!("Sending redeem transaction...");

    let contract_address = Address::from_str(contract)?;
    let to_address = Address::from_str(to)?;
    let secret_bytes = H256::from_str(secret)?;
    let salt_bytes = H256::from_str(salt)?;

    #[allow(deprecated)]
    let func = Function {
        name: "redeem".to_string(),
        inputs: vec![
            Param { name: "secret".into(), kind: ParamType::FixedBytes(32), internal_type: None },
            Param { name: "salt".into(), kind: ParamType::FixedBytes(32), internal_type: None },
            Param { name: "to".into(), kind: ParamType::Address, internal_type: None },
        ],
        outputs: vec![],
        state_mutability: StateMutability::NonPayable,
        constant: None,
    };
    let data = func.encode_input(&[
        Token::FixedBytes(secret_bytes.as_bytes().to_vec()),
        Token::FixedBytes(salt_bytes.as_bytes().to_vec()),
        Token::Address(to_address),
    ])?;
    let tx = TransactionRequest::new().to(contract_address).data(data);
    let pending_tx = client.send_transaction(tx, None).await?;
    let receipt = pending_tx.await?;

    match receipt {
        Some(r) => println!("Redeem TX confirmed: https://sepolia.etherscan.io/tx/{:?}", r.transaction_hash),
        None => println!("⏳ Redeem TX sent but not confirmed."),
    }

    Ok(())
}

async fn handle_refund(secret: &str, salt: &str, contract: &str) -> anyhow::Result<()> {
    let private_key = env::var("PRIVATE_KEY").expect("Missing PRIVATE_KEY");
    let rpc_url = env::var("RPC_URL").expect("Missing RPC_URL");

    let provider = Provider::<Http>::try_from(rpc_url)?;
    let chain_id = provider.get_chainid().await?.as_u64();
    let wallet: LocalWallet = private_key.parse::<LocalWallet>()?.with_chain_id(chain_id);
    let client = SignerMiddleware::new(provider, wallet);
    let client = Arc::new(client);

    println!("Sending refund transaction...");

    let contract_address = Address::from_str(contract)?;
    let secret_bytes = H256::from_str(secret)?;
    let salt_bytes = H256::from_str(salt)?;

    #[allow(deprecated)]
    let func = Function {
        name: "refund".to_string(),
        inputs: vec![
            Param { name: "secret".into(), kind: ParamType::FixedBytes(32), internal_type: None },
            Param { name: "salt".into(), kind: ParamType::FixedBytes(32), internal_type: None },
        ],
        outputs: vec![],
        state_mutability: StateMutability::NonPayable,
        constant: None,
    };
    let data = func.encode_input(&[
        Token::FixedBytes(secret_bytes.as_bytes().to_vec()),
        Token::FixedBytes(salt_bytes.as_bytes().to_vec()),
    ])?;
    let tx = TransactionRequest::new().to(contract_address).data(data);
    let pending_tx = client.send_transaction(tx, None).await?;
    let receipt = pending_tx.await?;

    match receipt {
        Some(r) => println!("Refund TX confirmed: https://sepolia.etherscan.io/tx/{:?}", r.transaction_hash),
        None => println!("Refund TX sent but not confirmed."),
    }

    Ok(())
}

// ---------------- HELPERS ----------------

fn make_commitment(expiry: &str) -> anyhow::Result<(String, String, String, u64, chrono::DateTime<Utc>)> {
    let mut rng = rand::thread_rng();
    let mut secret = [0u8; 32];
    let mut salt = [0u8; 32];
    rng.fill_bytes(&mut secret);
    rng.fill_bytes(&mut salt);

    let secret_hex = format!("0x{}", hex::encode(secret));
    let salt_hex = format!("0x{}", hex::encode(salt));
    let input = [secret.as_ref(), salt.as_ref()].concat();
    let commitment = keccak256(&input);

    let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
    let expiry_secs = parse_duration(expiry)?;
    let expiry_unix = now + expiry_secs;
    let expiry_date = Utc.timestamp_opt(expiry_unix as i64, 0).unwrap();

    Ok((secret_hex, salt_hex, commitment, expiry_unix, expiry_date))
}

// ABI encoding for: depositETH(bytes32,uint256,bool)
fn encode_deposit_eth(commitment: H256, expiry: u64, refundable: bool) -> Bytes {
    let tokens = vec![
        Token::FixedBytes(commitment.as_bytes().to_vec()),
        Token::Uint(expiry.into()),
        Token::Bool(refundable),
    ];

    #[allow(deprecated)]
    let func = Function {
        name: "depositETH".to_string(),
        inputs: vec![
            Param { name: "commitment".to_string(), kind: ParamType::FixedBytes(32), internal_type: None },
            Param { name: "expiry".to_string(), kind: ParamType::Uint(256), internal_type: None },
            Param { name: "refundable".to_string(), kind: ParamType::Bool, internal_type: None },
        ],
        outputs: vec![],
        state_mutability: StateMutability::Payable,
        constant: None,
    };

    func.encode_input(&tokens).unwrap().into()
}

// Generate QR code (ASCII + PNG)
fn generate_qr(secret: &str, salt: &str, contract: &str) -> anyhow::Result<()> {
    let url = format!(
        "https://somethingcorrosive.github.io/vaporpay/?secret={}&salt={}&contract={}",
        secret, salt, contract
    );

    println!("\n🔗 Redeem URL: {}", url);

    // ASCII QR in terminal
    let code = QrCode::new(url.as_bytes()).map_err(|e| anyhow::anyhow!(e))?;
    let string = code
        .render::<char>()
        .quiet_zone(true)
        .module_dimensions(2, 1)
        .build();
    println!("\n📱 QR Code (ASCII):\n{}", string);

    // PNG QR
    let scale = 10;
    let qr_width = code.width();
    let img_size = qr_width * scale;
    let mut img = ImageBuffer::new(img_size as u32, img_size as u32);

    for y in 0..qr_width {
        for x in 0..qr_width {
            let color = if code[(x, y)] == Color::Dark { 0u8 } else { 255u8 }; // 👈 fixed: compare Color
            for dy in 0..scale {
                for dx in 0..scale {
                    img.put_pixel(
                        (x * scale + dx) as u32,
                        (y * scale + dy) as u32,
                        Luma([color]),
                    );
                }
            }
        }
    }

    img.save("voucher.png")?;
    println!("Saved QR to voucher.png");

    Ok(())
}
