use std::str;
use zeromq::ZmqMessage;

#[derive(Debug, Clone, Copy)]
pub enum BitcoinEventType {
    RawTx,
    HashTx,
    RawBlock,
    HashBlock,
}

impl TryFrom<&str> for BitcoinEventType {
    type Error = String;

    fn try_from(event_type: &str) -> Result<Self, Self::Error> {
        match event_type {
            "rawtx" => Ok(BitcoinEventType::RawTx),
            "hashtx" => Ok(BitcoinEventType::HashTx),
            "rawblock" => Ok(BitcoinEventType::RawBlock),
            "hashblock" => Ok(BitcoinEventType::HashBlock),
            _ => Err(format!("Unknown event type: {}", event_type)),
        }
    }
}

#[derive(Clone)]
pub struct BitcoinEvent {
    data: Vec<u8>,
    event_type: BitcoinEventType,
    sequence_number: u32,
}

impl TryFrom<ZmqMessage> for BitcoinEvent {
    type Error = Box<dyn std::error::Error>;

    fn try_from(message: ZmqMessage) -> Result<Self, Self::Error> {
        let event_type_str: String = String::from_utf8(message.get(0).unwrap().to_vec())?;
        let event_type = BitcoinEventType::try_from(event_type_str.as_str())?;

        let data: Vec<u8> = message.get(1).unwrap().to_vec();
        let sequence_number_bytes: [u8; 4] = message.get(2).unwrap()[0..4].try_into()?;
        let sequence_number = u32::from_le_bytes(sequence_number_bytes);

        Ok(BitcoinEvent {
            event_type,
            data,
            sequence_number,
        })
    }
}


impl std::fmt::Debug for BitcoinEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BitcoinEvent")
            .field("type", &self.event_type)
            .field("data", &self.data)
            .field("sequence", &self.sequence_number)
            .finish()
    }
}

impl BitcoinEvent {
    pub fn get_type(&self) -> &BitcoinEventType {
        &self.event_type
    }

    pub fn get_data(&self) -> &Vec<u8> {
        &self.data
    }

    pub fn get_sequence_number(&self) -> u32 {
        self.sequence_number
    }
}
