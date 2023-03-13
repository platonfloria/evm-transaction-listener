# evm-transaction-listener
Service that listens for evm transactions and prints those to the console.

## configuration
Create an .env file with the following variable:
```
RPC_WS_URL=wss://mainnet.infura.io/ws/v3/<infura_api_key>
```
You can get infura_api_key from here https://app.infura.io.

## build
```bash
cargo build
```

## run
```bash
cargo run
```
or
```bash
docker-compose up
```