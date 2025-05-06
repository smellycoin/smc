use smchash::{SMCHash, Block, hash_to_hex};
use std::time::{Duration, Instant};
use std::sync::{Arc, Mutex};
use std::thread;

// Constants for the benchmark
const MAX_RUNTIME_SECONDS: u64 = 3;  // Max runtime of 3 seconds
const TRANSACTION_COUNT_PER_BLOCK: usize = 10;  // Reduced transaction count
const NUM_THREADS: usize = 2;  // Reduced thread count
const DIFFICULTY: u8 = 4;      // Reduced difficulty for faster mining

// Simple transaction structure
#[derive(Clone)]
struct Transaction {
    from: [u8; 16],
    to: [u8; 16],
    amount: u64,
    nonce: u64,
}

impl Transaction {
    fn new(from: [u8; 16], to: [u8; 16], amount: u64, nonce: u64) -> Self {
        Self { from, to, amount, nonce }
    }
    
    fn serialize(&self) -> Vec<u8> {
        let mut data = Vec::with_capacity(48);
        data.extend_from_slice(&self.from);
        data.extend_from_slice(&self.to);
        data.extend_from_slice(&self.amount.to_le_bytes());
        data.extend_from_slice(&self.nonce.to_le_bytes());
        data
    }
}

// Extended Block for our benchmark
struct BlockchainBlock {
    block: Block,
    transactions: Vec<Transaction>,
    block_num: usize,
}

impl BlockchainBlock {
    fn new(prev_hash: [u8; 16], transactions: Vec<Transaction>, timestamp: u64, block_num: usize) -> Self {
        // Serialize transactions
        let mut tx_data = Vec::new();
        for tx in &transactions {
            tx_data.extend_from_slice(&tx.serialize());
        }
        
        // Create block
        let block = Block::new(prev_hash, tx_data, timestamp, DIFFICULTY);
        
        Self {
            block,
            transactions,
            block_num,
        }
    }
}

fn main() {
    println!("Starting SMCHash Blockchain Mining Benchmark");
    println!("============================================");
    println!("Mining blocks with {} tx per block using {} threads", 
             TRANSACTION_COUNT_PER_BLOCK, NUM_THREADS);
    println!("Difficulty: {}", DIFFICULTY);
    println!("Max runtime: {} seconds", MAX_RUNTIME_SECONDS);

    // Create a genesis block
    let genesis_block = create_genesis_block();
    println!("Genesis block created!");
    
    // Create blockchain
    let blockchain = Arc::new(Mutex::new(vec![genesis_block]));
    let mining_times = Arc::new(Mutex::new(Vec::new()));
    let verification_times = Arc::new(Mutex::new(Vec::new()));
    
    let start_time = Instant::now();
    let should_continue = Arc::new(Mutex::new(true));
    
    // Create a thread to monitor execution time
    let should_continue_clone = Arc::clone(&should_continue);
    thread::spawn(move || {
        thread::sleep(Duration::from_secs(MAX_RUNTIME_SECONDS));
        let mut continue_flag = should_continue_clone.lock().unwrap();
        *continue_flag = false;
        println!("Time limit reached, stopping mining...");
    });
    
    // Mine blocks in parallel until time limit
    let mut handles = vec![];
    let mut block_num = 1;
    
    for thread_id in 0..NUM_THREADS {
        let blockchain_clone = Arc::clone(&blockchain);
        let mining_times_clone = Arc::clone(&mining_times);
        let verification_times_clone = Arc::clone(&verification_times);
        let should_continue_clone = Arc::clone(&should_continue);
        
        let handle = thread::spawn(move || {
            let mut blocks_mined = 0;
            
            while *should_continue_clone.lock().unwrap() {
                let mut prev_hash = [0u8; 16];
                
                // Get the last block's hash
                {
                    let chain = blockchain_clone.lock().unwrap();
                    prev_hash = chain.last().unwrap().block.hash;
                }
                
                // Create transactions
                let transactions = create_random_transactions(TRANSACTION_COUNT_PER_BLOCK);
                
                // Time the mining process
                let mining_start = Instant::now();
                let new_block = mine_block(prev_hash, transactions, thread_id * 1000 + blocks_mined);
                let mining_time = mining_start.elapsed();
                
                // Time the verification process
                let verification_start = Instant::now();
                let is_valid = new_block.block.validate(DIFFICULTY);
                let verification_time = verification_start.elapsed();
                
                // Check if we should still continue
                if !*should_continue_clone.lock().unwrap() {
                    break;
                }
                
                // Store the times
                mining_times_clone.lock().unwrap().push(mining_time);
                verification_times_clone.lock().unwrap().push(verification_time);
                
                // Add block to blockchain
                let mut chain = blockchain_clone.lock().unwrap();
                chain.push(new_block);
                
                blocks_mined += 1;
                println!("Thread {} mined block {} in {:?}", thread_id, blocks_mined, mining_time);
            }
            
            blocks_mined
        });
        
        handles.push(handle);
    }
    
    // Wait for all mining to complete
    let mut total_blocks = 0;
    for (i, handle) in handles.into_iter().enumerate() {
        let blocks_mined = handle.join().unwrap();
        total_blocks += blocks_mined;
        println!("Thread {} mined {} blocks", i, blocks_mined);
    }
    
    let total_time = start_time.elapsed();
    println!("Total blocks mined: {}", total_blocks);
    
    // Print blockchain
    println!("\nFinal Blockchain");
    println!("================");
    let chain = blockchain.lock().unwrap();
    for (i, block) in chain.iter().enumerate() {
        println!("Block {} - Hash: {}", i, hash_to_hex(&block.block.hash));
    }
    
    // Calculate average mining and verification times
    let mining_times = mining_times.lock().unwrap();
    let verification_times = verification_times.lock().unwrap();
    
    let avg_mining_time: Duration = mining_times.iter().sum::<Duration>() / mining_times.len() as u32;
    let avg_verification_time: Duration = verification_times.iter().sum::<Duration>() / verification_times.len() as u32;
    
    println!("\nPerformance Summary");
    println!("===================");
    println!("Total time: {:?}", total_time);
    println!("Avg mining time: {:?} per block", avg_mining_time);
    println!("Avg verification time: {:?} per block", avg_verification_time);
    println!("Blocks per second: {:.2}", BLOCK_COUNT as f64 / total_time.as_secs_f64());
    println!("Transactions per second: {:.2}", 
             (BLOCK_COUNT * TRANSACTION_COUNT_PER_BLOCK) as f64 / total_time.as_secs_f64());
    
    // Revalidate the entire blockchain
    println!("\nRevalidating entire blockchain...");
    let validation_start = Instant::now();
    let mut is_valid = true;
    for i in 1..chain.len() {
        let prev_hash = chain[i-1].block.hash;
        let current_block = &chain[i].block;
        
        // Validate block hash
        if !current_block.validate(DIFFICULTY) {
            println!("Block {} has invalid hash!", i);
            is_valid = false;
            break;
        }
        
        // Validate block links
        if current_block.prev_hash != prev_hash {
            println!("Block {} has invalid previous hash link!", i);
            is_valid = false;
            break;
        }
    }
    
    let validation_time = validation_start.elapsed();
    println!("Entire blockchain valid: {}", is_valid);
    println!("Full validation time: {:?}", validation_time);
    println!("Validation speed: {:.2} blocks per second", 
             chain.len() as f64 / validation_time.as_secs_f64());
}

fn create_genesis_block() -> BlockchainBlock {
    let prev_hash = [0u8; 16];
    let timestamp = get_timestamp();
    
    BlockchainBlock::new(
        prev_hash,
        vec![create_coinbase_transaction()],
        timestamp,
        0
    )
}

fn mine_block(prev_hash: [u8; 16], transactions: Vec<Transaction>, block_num: usize) -> BlockchainBlock {
    let timestamp = get_timestamp();
    BlockchainBlock::new(prev_hash, transactions, timestamp, block_num)
}

fn create_random_transactions(count: usize) -> Vec<Transaction> {
    let mut transactions = Vec::with_capacity(count + 1);
    
    // Add a coinbase transaction first
    transactions.push(create_coinbase_transaction());
    
    // Add regular transactions
    for i in 0..count {
        let from = generate_random_address();
        let to = generate_random_address();
        let amount = (i as u64 + 1) * 100;
        let nonce = i as u64;
        
        transactions.push(Transaction::new(from, to, amount, nonce));
    }
    
    transactions
}

fn create_coinbase_transaction() -> Transaction {
    let zero_address = [0u8; 16];
    let miner_address = generate_random_address();
    
    Transaction::new(zero_address, miner_address, 5000, 0)
}

fn generate_random_address() -> [u8; 16] {
    let mut address = [0u8; 16];
    for i in 0..16 {
        address[i] = (std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos() % 256) as u8;
        
        // Add some entropy
        thread::sleep(Duration::from_nanos(1));
    }
    address
}

fn get_timestamp() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs()
}