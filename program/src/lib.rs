use solana_program::declare_id;

pub use crate::instruction::Instruction;

pub mod instruction;
pub mod processor;
pub mod state;

#[cfg(not(feature = "no-entrypoint"))]
mod entrypoint;

declare_id!("GaDuBQ9poRLs8vDYQTQQqcyYfSSvYqXqGDB9JCtgNyA5");
