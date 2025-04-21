use solana_program::declare_id;

pub use crate::instruction::Instruction;

pub mod instruction;
pub mod processor;
pub mod state;

#[cfg(not(feature = "no-entrypoint"))]
mod entrypoint;

declare_id!("6NYZcL1SDYURfnNFZVNj6qGo5rpqeL1SAT9nHjzq2WzM");
