use uuid::{Uuid, uuid};

use crate::MemoryItemUrn;

pub trait IdGenerator: Send + Sync {
    fn new_uuid(&self) -> Uuid;

    fn memory_item_urn(
        &self,
        source_id: Uuid,
        sequence: u32,
        start_offset: u32,
        end_offset: u32,
        content_hash: &str,
    ) -> MemoryItemUrn;
}

#[derive(Debug, Default)]
pub struct DefaultIdGenerator;

const MEMORY_ITEM_NAMESPACE: Uuid = uuid!("d1f0da4e-a281-55af-b8c4-315af97a5ecf");

impl IdGenerator for DefaultIdGenerator {
    fn new_uuid(&self) -> Uuid {
        Uuid::new_v4()
    }

    fn memory_item_urn(
        &self,
        source_id: Uuid,
        sequence: u32,
        start_offset: u32,
        end_offset: u32,
        content_hash: &str,
    ) -> MemoryItemUrn {
        let seed = format!("{source_id}:{sequence}:{start_offset}:{end_offset}:{content_hash}");
        MemoryItemUrn::new(format!(
            "urn:memory-item:{}",
            Uuid::new_v5(&MEMORY_ITEM_NAMESPACE, seed.as_bytes())
        ))
    }
}
