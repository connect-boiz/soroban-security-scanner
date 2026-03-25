use rand::{Rng, thread_rng, distributions::Alphanumeric};
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FuzzValue {
    U32(u32),
    I32(i32),
    Bool(bool),
    Bytes(Vec<u8>),
    String(String),
    Symbol(String),
    Address(String),
    Vector(Vec<FuzzValue>),
}

pub struct InputGenerator;

impl InputGenerator {
    pub fn random_u32() -> u32 {
        thread_rng().gen()
    }

    pub fn random_i32() -> i32 {
        thread_rng().gen()
    }

    pub fn random_bool() -> bool {
        thread_rng().gen()
    }

    pub fn random_bytes(len: usize) -> Vec<u8> {
        let mut rng = thread_rng();
        (0..len).map(|_| rng.gen()).collect()
    }

    pub fn random_string(len: usize) -> String {
        thread_rng()
            .sample_iter(&Alphanumeric)
            .take(len)
            .map(char::from)
            .collect()
    }

    pub fn random_symbol() -> String {
        // Symbols in Soroban are max 32 chars, alphanumeric and underscores
        let len = thread_rng().gen_range(1..32);
        Self::random_string(len)
    }

    pub fn random_address() -> String {
        // Mock G-address
        format!("G{}", Self::random_string(55))
    }

    pub fn generate_random_input() -> FuzzValue {
        let mut rng = thread_rng();
        match rng.gen_range(0..8) {
            0 => FuzzValue::U32(Self::random_u32()),
            1 => FuzzValue::I32(Self::random_i32()),
            2 => FuzzValue::Bool(Self::random_bool()),
            3 => FuzzValue::Bytes(Self::random_bytes(rng.gen_range(0..64))),
            4 => FuzzValue::String(Self::random_string(rng.gen_range(0..32))),
            5 => FuzzValue::Symbol(Self::random_symbol()),
            6 => FuzzValue::Address(Self::random_address()),
            _ => {
                let vec_len = rng.gen_range(0..5);
                FuzzValue::Vector((0..vec_len).map(|_| Self::generate_random_input()).collect())
            }
        }
    }
}
