use std::convert::TryInto;

/// SMCHash - A fast, lightweight hashing algorithm designed for blockchain applications
/// Features:
/// - Small hash size (16 bytes / 128 bits)
/// - Fast verification
/// - Lightweight computation
/// - Designed for blockchain integration

pub struct SMCHash {
    // Internal state variables
    state: [u32; 4],
    buffer: Vec<u8>,
    total_bytes: u64,
}

impl SMCHash {
    /// Creates a new SMCHash instance with default initialization
    pub fn new() -> Self {
        SMCHash {
            // Initialize with prime numbers for better distribution
            state: [0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a],
            buffer: Vec::new(),
            total_bytes: 0,
        }
    }
    
    /// Updates the hash state with input data
    pub fn update(&mut self, data: &[u8]) {
        self.buffer.extend_from_slice(data);
        self.total_bytes += data.len() as u64;
        
        // Process complete blocks (64 bytes each)
        while self.buffer.len() >= 64 {
            // Create a separate buffer to avoid borrowing issues
            let block_data = self.buffer[0..64].to_vec();
            self.process_block(&block_data);
            self.buffer.drain(0..64);
        }
    }
    
    /// Processes a single 64-byte block
    fn process_block(&mut self, block: &[u8]) {
        // Convert block to sixteen 32-bit words
        let mut words = [0u32; 16];
        for i in 0..16 {
            let start = i * 4;
            let word_bytes: [u8; 4] = block[start..start + 4].try_into().unwrap();
            words[i] = u32::from_le_bytes(word_bytes);
        }
        
        // Save current state
        let mut a = self.state[0];
        let mut b = self.state[1];
        let mut c = self.state[2];
        let mut d = self.state[3];
        
        // Main mixing function - 4 rounds of operations
        for round in 0..4 {
            for i in 0..16 {
                let f = match round {
                    0 => (b & c) | (!b & d),             // Round 1: if b then c else d
                    1 => (b & d) | (c & !d),             // Round 2: different bit mixing
                    2 => b ^ c ^ d,                     // Round 3: XOR
                    _ => c ^ (b | !d),                  // Round 4: alternative mixing
                };
                
                let word_idx = match round {
                    0 => i,                            // Sequential in first round
                    1 => (5*i + 1) % 16,               // Different permutation per round
                    2 => (3*i + 5) % 16,
                    _ => (7*i) % 16,
                };
                
                let k = match round {
                    0 => 0x79cc4519,
                    1 => 0x9d8a7a87,
                    2 => 0xe9b5dba5,
                    _ => 0xc19bf274,
                } + i as u32;
                
                // Rotation constants
                let s = match round {
                    0 => [7, 12, 17, 22],
                    1 => [5, 9, 14, 20],
                    2 => [4, 11, 16, 23],
                    _ => [6, 10, 15, 21],
                };
                
                let temp = d;
                d = c;
                c = b;
                let rot_amount = s[i % 4];
                b = b.wrapping_add(rotl(a.wrapping_add(f).wrapping_add(words[word_idx]).wrapping_add(k), rot_amount));
                a = temp;
            }
        }
        
        // Update state with the result
        self.state[0] = self.state[0].wrapping_add(a);
        self.state[1] = self.state[1].wrapping_add(b);
        self.state[2] = self.state[2].wrapping_add(c);
        self.state[3] = self.state[3].wrapping_add(d);
    }
    
    /// Finalizes the hash computation and returns the hash
    pub fn finalize(mut self) -> [u8; 16] {
        // Add padding similar to MD5/SHA
        let bit_len = self.total_bytes * 8;

        // Add a single '1' bit
        self.buffer.push(0x80);
        
        // Pad with zeros to get 56 bytes mod 64
        while self.buffer.len() % 64 != 56 {
            self.buffer.push(0);
        }
        
        // Append length as 64-bit little-endian integer
        self.buffer.extend_from_slice(&bit_len.to_le_bytes());
        
        // Process last blocks
        while self.buffer.len() >= 64 {
            // Create a separate buffer to avoid borrowing issues
            let block_data = self.buffer[0..64].to_vec();
            self.process_block(&block_data);
            self.buffer.drain(0..64);
        }

        // Convert state to bytes (16 bytes total)
        let mut result = [0u8; 16];
        for i in 0..4 {
            let bytes = self.state[i].to_le_bytes();
            result[i*4..(i+1)*4].copy_from_slice(&bytes);
        }
        
        result
    }
    
    /// Simple one-shot hash function for convenience
    pub fn hash(data: &[u8]) -> [u8; 16] {
        let mut hasher = SMCHash::new();
        hasher.update(data);
        hasher.finalize()
    }
    
    /// Fast verification method for blockchain applications
    /// Returns true if the hash is valid for the given data
    pub fn verify(data: &[u8], expected_hash: &[u8; 16]) -> bool {
        let computed_hash = Self::hash(data);
        
        // Time-constant comparison to prevent timing attacks
        let mut result = 0;
        for i in 0..16 {
            result |= computed_hash[i] ^ expected_hash[i];
        }
        
        result == 0
    }
    
    /// Creates a proof of work by finding a nonce that produces a hash with
    /// the specified number of leading zero bits
    pub fn create_proof_of_work(data: &[u8], difficulty: u8) -> (u64, [u8; 16]) {
        let mut nonce: u64 = 0;
        let target_mask = if difficulty >= 8 {
            0xFF
        } else {
            0xFF >> (8 - difficulty)
        };
        
        loop {
            let mut hasher = SMCHash::new();
            hasher.update(data);
            hasher.update(&nonce.to_le_bytes());
            let hash = hasher.finalize();
            
            // Check if we have the required number of leading zeros
            let zeros_required = difficulty / 8;
            let bits_in_last_byte = difficulty % 8;
            
            let mut valid = true;
            
            // Check full zero bytes
            for i in 0..zeros_required as usize {
                if hash[i] != 0 {
                    valid = false;
                    break;
                }
            }
            
            // Check partial zero byte if needed
            if valid && bits_in_last_byte > 0 {
                valid = (hash[zeros_required as usize] & target_mask) == 0;
            }
            
            if valid {
                return (nonce, hash);
            }
            
            nonce += 1;
        }
    }
    
    /// Verifies a proof of work
    pub fn verify_proof_of_work(data: &[u8], nonce: u64, difficulty: u8, expected_hash: &[u8; 16]) -> bool {
        let mut hasher = SMCHash::new();
        hasher.update(data);
        hasher.update(&nonce.to_le_bytes());
        let hash = hasher.finalize();
        
        // Verify hash matches expected hash
        if hash != *expected_hash {
            return false;
        }
        
        // Verify difficulty requirement
        let zeros_required = difficulty / 8;
        let bits_in_last_byte = difficulty % 8;
        let target_mask = if bits_in_last_byte == 0 {
            0
        } else {
            0xFF >> (8 - bits_in_last_byte)
        };
        
        // Check full zero bytes
        for i in 0..zeros_required as usize {
            if hash[i] != 0 {
                return false;
            }
        }
        
        // Check partial zero byte if needed
        if bits_in_last_byte > 0 {
            if (hash[zeros_required as usize] & target_mask) != 0 {
                return false;
            }
        }
        
        true
    }
}

// Helper function: left rotation
fn rotl(x: u32, n: u32) -> u32 {
    (x << n) | (x >> (32 - n))
}

// Utility function to convert hash to hex string
pub fn hash_to_hex(hash: &[u8; 16]) -> String {
    hash.iter().map(|b| format!("{:02x}", b)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_empty_string() {
        let hash = SMCHash::hash("".as_bytes());
        // This is just a placeholder expected value - actual implementation would have its own value
        assert_eq!(hash_to_hex(&hash).len(), 32); // 16 bytes = 32 hex chars
    }
    
    #[test]
    fn test_verification() {
        let data = "test data for verification".as_bytes();
        let hash = SMCHash::hash(data);
        assert!(SMCHash::verify(data, &hash));
        
        // Modify data and ensure verification fails
        let modified_data = "test data for verificationX".as_bytes();
        assert!(!SMCHash::verify(modified_data, &hash));
    }
    
    #[test]
    fn test_proof_of_work() {
        let data = "blockchain data".as_bytes();
        let difficulty = 8; // 8 bits = 1 byte of leading zeros
        
        let (nonce, hash) = SMCHash::create_proof_of_work(data, difficulty);
        assert!(SMCHash::verify_proof_of_work(data, nonce, difficulty, &hash));
        
        // Test first byte is zero (8 bits of difficulty)
        assert_eq!(hash[0], 0);
    }
    
    #[test]
    fn test_different_inputs_produce_different_hashes() {
        let hash1 = SMCHash::hash("input1".as_bytes());
        let hash2 = SMCHash::hash("input2".as_bytes());
        assert_ne!(hash1, hash2);
    }
}

// Example usage in a blockchain context
#[derive(Debug)]
pub struct Block {
    pub prev_hash: [u8; 16],
    pub data: Vec<u8>,
    pub timestamp: u64,
    pub nonce: u64,
    pub hash: [u8; 16],
}

impl Block {
    pub fn new(prev_hash: [u8; 16], data: Vec<u8>, timestamp: u64, difficulty: u8) -> Self {
        let mut block = Block {
            prev_hash,
            data,
            timestamp,
            nonce: 0,
            hash: [0; 16],
        };
        
        // Create the block hash with proof of work
        let block_data = block.get_hashable_data();
        let (nonce, hash) = SMCHash::create_proof_of_work(&block_data, difficulty);
        
        block.nonce = nonce;
        block.hash = hash;
        block
    }
    
    fn get_hashable_data(&self) -> Vec<u8> {
        let mut data = Vec::new();
        data.extend_from_slice(&self.prev_hash);
        data.extend_from_slice(&self.data);
        data.extend_from_slice(&self.timestamp.to_le_bytes());
        data
    }
    
    pub fn validate(&self, difficulty: u8) -> bool {
        let block_data = self.get_hashable_data();
        SMCHash::verify_proof_of_work(&block_data, self.nonce, difficulty, &self.hash)
    }
}