use std::{env, time::{Duration, SystemTime}, thread, collections::HashSet, sync::Arc, str::FromStr};

use eyre::{bail, eyre, Result};
use chrono::{DateTime, Utc};
use tokio::sync::{mpsc::{self, UnboundedSender, UnboundedReceiver}, Mutex};
use web3::{
    futures::StreamExt,
    transports::WebSocket,
    types::{Address, BlockHeader, BlockId, BlockNumber, Transaction},
    Web3
};


struct Controller {
    web3: web3::Web3<WebSocket>,
    watched_addresses: Mutex<HashSet<Address>>,
}


fn iso8601(st: &std::time::SystemTime) -> String {
    let dt: DateTime<Utc> = st.clone().into();
    format!("{}", dt.format("%+"))
    // formats like "2001-07-08T00:34:60.026490+09:30"
}


impl Controller {
    pub fn new(web3: web3::Web3<WebSocket>) -> Result<Self> {
        Ok(Controller {
            web3,
            watched_addresses: Mutex::new(HashSet::new()),
        })
    }

    async fn listen_to_new_blocks(&self, block_sender: UnboundedSender<(SystemTime, BlockHeader)>) -> web3::contract::Result<()> {
        let sub = self.web3.eth_subscribe().subscribe_new_heads().await?;

        sub.fold(block_sender, |block_sender, log| async move {
            if let Ok(block_header) = log {
                let timestamp = SystemTime::now();
                block_sender.send((timestamp, block_header)).expect("failed to push log");
            }

            block_sender
        }).await;

        Ok(())
    }

    async fn process_new_blocks(self: Arc<Self>, mut block_receiver: UnboundedReceiver<(SystemTime, BlockHeader)>) -> Result<()> {
        loop {
            if let Some((timestamp, block_header)) = block_receiver.recv().await {
                println!("{:?}", block_header);

                let block_number = block_header.number.ok_or(eyre!("header does not contain block number"))?;
                let block = self.web3.eth().block_with_txs(BlockId::Number(BlockNumber::Number(block_number))).await?.ok_or(eyre!("unable to get the block"))?;
                let mut transactions = block.transactions;
                self.filter_relevant_transactions(&mut transactions).await;
                for transaction in transactions {
                    self.process_transaction(timestamp, transaction).await?;
                }
            }
        }
    }

    async fn filter_relevant_transactions(&self, transactions: &mut Vec<Transaction>) {
        let watched_addresses = self.watched_addresses.lock().await;
        transactions.retain(|transaction| {
            let transaction_addresses = [transaction.from, transaction.to].into_iter().flatten().collect();
            watched_addresses.intersection(&transaction_addresses).count() > 0
        });
    }

    async fn process_transaction(&self, timestamp: SystemTime, transaction: Transaction) -> Result<()> {
        println!("{:?} {:?}", iso8601(&timestamp), transaction);
        Ok(())
    }

    async fn sync_watched_addresses(self: Arc<Self>) -> Result<()> {
        self.watched_addresses.lock().await.extend([
            Address::from_str("0xe592427a0aece92de3edee1f18e0157c05861564")?,
            Address::from_str("0xdAC17F958D2ee523a2206206994597C13D831ec7")?,
        ].iter());
        
        Ok(())
    }
}


async fn start() -> Result<()> {
    let websocket = WebSocket::new(&env::var("RPC_WS_URL").expect("RPC_WS_URL env variable not set")).await?;
    let web3 = Web3::new(websocket.clone());

    let (block_sender, block_receiver) = mpsc::unbounded_channel();

    let controller = Arc::new(Controller::new(web3)?);

    match tokio::join!(
        controller.listen_to_new_blocks(block_sender),
        controller.clone().process_new_blocks(block_receiver),
        controller.clone().sync_watched_addresses(),
    ) {
        (Ok(_), Ok(_), Ok(_)) => Ok(()),
        _ => bail!("service has crashed"),
    }
}


#[tokio::main]
async fn main() -> Result<()> {
    dotenv::dotenv().ok();
    let _ = env_logger::try_init();

    loop {
        match start().await {
            Ok(_) => break,
            Err(e) => {
                println!("Error: {:?}", e);
                println!("Restarting in 5 seconds...");
                let ten_millis = Duration::from_secs(5);
                thread::sleep(ten_millis);
            }
        }
    }

    Ok(())
}