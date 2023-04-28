use zeromq::ZmqMessage;

#[derive(Debug)]
pub enum BitcoinEventType {
    RawTx,
    HashTx,
    RawBlock,
    HashBlock,
    Unknown,
}

impl From<&str> for BitcoinEventType {
    fn from(event_type: &str) -> Self {
        match event_type {
            "rawtx" => BitcoinEventType::RawTx,
            "hashtx" => BitcoinEventType::HashTx,
            "rawblock" => BitcoinEventType::RawBlock,
            "hashblock" => BitcoinEventType::HashBlock,
            _ => BitcoinEventType::Unknown,
        }
    }
}

impl BitcoinEventType {
    pub fn as_str(&self) -> &str {
        match self {
            BitcoinEventType::RawTx => "rawtx",
            BitcoinEventType::HashTx => "hashtx",
            BitcoinEventType::RawBlock => "rawblock",
            BitcoinEventType::HashBlock => "hashblock",
            BitcoinEventType::Unknown => "unknown",
        }
    }

    pub fn to_string(&self) -> String {
        self.as_str().to_string()
    }
}

#[derive(Clone)]
pub enum BitcoinEvent {
    RawTx(Vec<u8>),
    HashTx(Vec<u8>),
    RawBlock(Vec<u8>),
    HashBlock(Vec<u8>),
    Unknown(Vec<u8>),
}

impl TryFrom<ZmqMessage> for BitcoinEvent {
    type Error = Box<dyn std::error::Error>;

    fn try_from(message: ZmqMessage) -> Result<Self, Self::Error> {
        let event_type = String::from_utf8(message.get(0).unwrap().to_vec())?;
        let event_type = BitcoinEventType::from(event_type.as_str());

        let data: Vec<u8> = message.get(1).unwrap().to_vec();
        let sequence_number_bytes: [u8; 4] = message.get(2).unwrap()[0..4].try_into()?;
        let sequence_number = u32::from_le_bytes(sequence_number_bytes);

        let event = match event_type.as_str() {
            "rawtx" => BitcoinEvent::RawTx(data),
            "hashtx" => BitcoinEvent::HashTx(data),
            "rawblock" => BitcoinEvent::RawBlock(data),
            "hashblock" => BitcoinEvent::HashBlock(data),
            _ => BitcoinEvent::Unknown(data),
        };

        Ok(event)
    }
}

impl std::fmt::Debug for BitcoinEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BitcoinEvent")
            .field("type", &self.event_type())
            .field("data", &self.data())
            .finish()
    }
}

impl BitcoinEvent {
    pub fn event_type(&self) -> BitcoinEventType {
        match self {
            BitcoinEvent::RawTx(_) => BitcoinEventType::RawTx,
            BitcoinEvent::HashTx(_) => BitcoinEventType::HashTx,
            BitcoinEvent::RawBlock(_) => BitcoinEventType::RawBlock,
            BitcoinEvent::HashBlock(_) => BitcoinEventType::HashBlock,
            BitcoinEvent::Unknown(_) => BitcoinEventType::Unknown,
        }
    }

    pub fn data(&self) -> &Vec<u8> {
        match self {
            BitcoinEvent::RawTx(data) => data,
            BitcoinEvent::HashTx(data) => data,
            BitcoinEvent::RawBlock(data) => data,
            BitcoinEvent::HashBlock(data) => data,
            BitcoinEvent::Unknown(data) => data,
        }
    }
}
