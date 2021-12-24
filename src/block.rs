use chrono::Utc;
use log::{info, warn, error};
use sha2::Digest;
use serde::{Serialize, Deserialize};

const DIFFICULTY_PREFIX: &str = "00";


pub struct App {
    pub blocks: Vec<Block>,
}

impl App {
    pub fn new() -> Self {
        Self { blocks: vec![] }
    }

    pub fn genesis(&mut self) {
        let genesis_block = Block {
            id: 0,
            timestamp: Utc::now().timestamp(),
            previous_hash: String::from("genesis"),
            data: String::from("genesis!"),
            nonce: 2836,
            hash: "0000f816a87f806bb0073dcf026a64fb40c946b5abee2573702828694d5b4c43".to_string(),
        };

        self.blocks.push(genesis_block);
    }

    pub fn try_add_block(&mut self, block: Block) {
        let latest_block = self.blocks.last().expect("there is as least one block");
        if self.is_block_valid(&block, latest_block) {
            self.blocks.push(block);
        } else {
            error!("could not add block - invalid");
        }
    }

    pub fn choose_chain(&mut self, local: Vec<Block>, remote: Vec<Block>) -> Vec<Block> {
        let is_local_valid = self.is_chain_valid(&local);
        let is_remote_valid = self.is_chain_valid(&remote);

        if is_local_valid && is_remote_valid {
            if local.len() >= remote.len() {
                local
            } else {
                remote
            }
        } else if is_remote_valid {
            remote
        } else if is_local_valid {
            local
        } else {
            panic!("local and remote chains are both invalid");
        }
    }

    fn is_block_valid(&self, block: &Block, previous_block: &Block) -> bool {
        if block.previous_hash != previous_block.hash {
            warn!("block({}) has wrong previous hash", block.id);
            return false;
        } else if !hash_to_string(&hex::decode(&block.hash).expect("can decode from hex")).starts_with(DIFFICULTY_PREFIX) {
            warn!("block({}) has invalid difficulty", block.id);
            return false;
        } else if block.id != previous_block.id + 1 {
            warn!("block({}) is not the next block after the latest: {}", block.id, previous_block.id);
            return false;
        } else if block.is_hash_valid() {
            warn!("block({}) has invalid hash", block.id);
            return false;
        }
        true
    }

    fn is_chain_valid(&self, chain: &[Block]) -> bool {
        for i in 0..chain.len() {
            if i == 0 {
                continue;
            }
            let previous_block = chain.get(i - 1).expect("has to exist");
            let block = chain.get(i).expect("has to exist");

            if !self.is_block_valid(block, previous_block) {
                return false;
            }
        }
        true
    }
}


#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Block {
    pub id: u64,
    pub hash: String,
    pub previous_hash: String,
    pub timestamp: i64,
    pub data: String,
    pub nonce: u64,
}

impl Block {
    pub fn new(id: u64, previous_hash: String, data: String) -> Self {
        let timestamp = Utc::now().timestamp();
        let (nonce, hash) = Block::mine_block(id, timestamp, &previous_hash, &data);

        Self {
            id,
            hash,
            timestamp,
            previous_hash,
            data,
            nonce,
        }
    }

    fn is_hash_valid(&self) -> bool {
        let hash = hex::encode(Block::calculate_hash(self.id, self.timestamp, &self.previous_hash, &self.data, self.nonce));
        hash != self.hash
    }

    pub fn mine_block(id: u64, timestamp: i64, previous_hash: &str, data: &str) -> (u64, String) {
        info!("mining block...");
        let mut nonce = 0;

        loop {
            if nonce % 100000 == 0 {
                info!("nonce: {}", nonce);
            }
            let hash = Block::calculate_hash(id, timestamp, previous_hash, data, nonce);
            let binary_hash = hash_to_string(&hash);
            if binary_hash.starts_with(DIFFICULTY_PREFIX) {
                info!("mined! nonce: {}, hash: {}, binary hash: {}", nonce, hex::encode(&hash), binary_hash);
                return (nonce, hex::encode(hash));
            }
            nonce += 1;
        }
    }

    pub fn calculate_hash(id: u64, timestamp: i64, previous_hash: &str, data: &str, nonce: u64) -> Vec<u8> {
        let data = serde_json::json!({
            "id": id,
            "previous_hash": previous_hash,
            "data": data,
            "timestamp": timestamp,
            "nonce": nonce
        });
        let mut hasher = sha2::Sha256::new();
        hasher.update(data.to_string().as_bytes());
        hasher.finalize().as_slice().to_owned()
    }

}


fn hash_to_string(hash: &[u8]) -> String {
    let mut res: String = String::default();
    for c in hash {
        res.push_str(&format!("{:b}", c));
    }
    res
}

