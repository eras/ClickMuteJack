use std::sync::mpsc;

pub enum Message {
    UpdateConfig(crate::config::Config),
}

pub type Receiver = mpsc::Receiver<Message>;
pub type Sender = mpsc::Sender<Message>;
