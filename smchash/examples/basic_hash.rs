use smchash::{SMCHash, hash_to_hex, Block};

fn main() {
    // Test basic hashing
    let data = b"Hello SMCHash!";
    let hash = SMCHash::hash(data);
    println!("Hash of '{}': {}", std::str::from_utf8(data).unwrap(), hash_to_hex(&hash));
    
    // Test creating a block
    let block = Block::new([0; 16], data.to_vec(), 12345, 4);
    println!("Block hash: {}", hash_to_hex(&block.hash));
    println!("Block valid: {}", block.validate(4));
}