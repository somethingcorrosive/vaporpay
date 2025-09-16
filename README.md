# VaporPay

VaporPay is a minimal, privacy-preserving payment system built on Ethereum and compatible L2s. 
It uses hash-time-locked commitments (HTLCs) to let anyone create a voucher (ETH or ERC-20) that can be redeemed with a secret + salt.

---

## Features
-  Privacy by design (only secret + salt can unlock funds)
-  Expiry support (minutes → months)
-  ETH and ERC-20 tokens supported
-  Refundable or burn-forever modes
-  Works on Ethereum mainnet and EVM L2s

---

## Contracts

- **Ethereum Mainnet**:  
  [`0xF7e0902B045688c8dDBf75cdf51947a4ba3b1d8F`](https://etherscan.io/address/0xF7e0902B045688c8dDBf75cdf51947a4ba3b1d8F)

- **Sepolia Testnet**:  
  [`0xB8403504e576a56065ec0587c26082Ef478a65ba`](https://sepolia.etherscan.io/address/0xB8403504e576a56065ec0587c26082Ef478a65ba)

---

##  dApp

Try the redeem/refund dApp (works with MetaMask):

 [VaporPay Redeem (GitHub Pages)](https://somethingcorrosive.github.io/vaporpay-redeem/)

Paste in a QR-generated link like:


The page:
- Detects if you’re on **Mainnet** or **Sepolia**
- Shows voucher details (amount, token, expiry, refundable)
- Lets you Redeem or Refund with a single click

---

##  Usage

### Rust CLI (friendly)
Install [Rust](https://www.rust-lang.org/) and clone this repo:

```bash
cargo run -- create \
  --amount 0.1 \
  --expiry 10m \
  --refundable \
  --send \
  --contract 0xF7e0902B045688c8dDBf75cdf51947a4ba3b1d8F \
  --qr
```
This prints:

secret + salt

commitment

Redeem URL + QR code

```bash
cargo run -- refund \
  --secret 0xSECRET \
  --salt 0xSALT \
  --contract 0xF7e0902B045688c8dDBf75cdf51947a4ba3b1d8F
```

### Refund Example

```bash
cargo run -- refund \
  --secret 0xSECRET \
  --salt 0xSALT \
  --contract 0xF7e0902B045688c8dDBf75cdf51947a4ba3b1d8F
```

## Foundry Examples

### Deposit Eth

```bash
cast send 0xF7e0902B045688c8dDBf75cdf51947a4ba3b1d8F \
  "depositETH(bytes32,uint256,bool)" \
  0x<commitment> \
  $(($(date +%s)+600)) \
  true \
  --value 0.1ether \
  --rpc-url $MAINNET_RPC \
  --private-key $PRIVATE_KEY
```

### Deposit ERC-20
#### two steps

### Approve

```bash
cast send 0xTOKEN \
  "approve(address,uint256)" \
  0xF7e0902B045688c8dDBf75cdf51947a4ba3b1d8F \
  5000000 \
  --rpc-url $MAINNET_RPC \
  --private-key $PRIVATE_KEY
```

### Deposit
```bash
cast send 0xF7e0902B045688c8dDBf75cdf51947a4ba3b1d8F \
  "depositERC20(bytes32,address,uint256,uint256,bool)" \
  0x<commitment> \
  0xTOKEN \
  5000000 \
  $(($(date +%s)+600)) \
  true \
  --rpc-url $MAINNET_RPC \
  --private-key $PRIVATE_KEY
```

### Redeem
```bash
cast send 0xF7e0902B045688c8dDBf75cdf51947a4ba3b1d8F \
  "redeem(bytes32,bytes32,address)" \
  0x<secret> 0x<salt> 0xYOUR_WALLET \
  --rpc-url $MAINNET_RPC \
  --private-key $PRIVATE_KEY
```

### Refund
```bash
cast send 0xF7e0902B045688c8dDBf75cdf51947a4ba3b1d8F \
  "refund(bytes32,bytes32)" \
  0x<secret> 0x<salt> \
  --rpc-url $MAINNET_RPC \
  --private-key $PRIVATE_KEY
```

### Notes
expiry is a Unix timestamp (seconds). CLI supports 5m, 2h, 30d etc.

refundable = false → funds are permanently locked if not redeemed.

Anyone can trigger refund, but only depositor (if refundable) or no one (if non-refundable) actually receives the funds.

### Support
Want to support development?

Donate ETH or USDC (Mainnet) to:

```bash
0x01a7907a851A5E0282e88464056a8FA85CDE06f4
```
